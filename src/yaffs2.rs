mod consts;
mod inode;
mod object_header;
mod object_type;
mod tags;
mod tree;
pub mod util;

use std::{
    collections::HashMap,
    fs::File,
    io::{self, Seek},
    path::PathBuf,
    time::{Duration, SystemTime},
};

use consts::*;
use inode::*;
use object_header::*;
use object_type::ObjectType;
use tags::*;
use tree::*;

use fuser::{Errno, FileAttr, FileType, Filesystem, FopenFlags, Generation, INodeNo};
use log::{info, warn};

use crate::yaffs2::util::bread;

pub struct Yaffs2 {
    pub file: File,
    pub mtd_page: u64,
    pub mtd_extra: u64,
    pub mtd_erase: u64,
    pub chunks_per_block: u64,
    pub nblocks: u64,
    pub nchunks: u64,
    pub block_size: u64,
    pub object_map: HashMap<INodeNo, INode>,
    pub buffers: Vec<Vec<u8>>,
}

impl Yaffs2 {
    pub fn new(path: PathBuf) -> io::Result<Self> {
        match File::open(path) {
            Ok(file) => Ok(Self {
                file: file,
                mtd_page: Default::default(),
                mtd_extra: Default::default(),
                mtd_erase: Default::default(),
                chunks_per_block: Default::default(),
                nblocks: Default::default(),
                nchunks: Default::default(),
                block_size: Default::default(),
                object_map: Default::default(),
                buffers: Default::default(),
            }),
            Err(e) => Err(e),
        }
    }

    fn read_inode(&self, ino: INodeNo) -> Option<&INode> {
        self.object_map.get(&ino)
    }

    fn stat(&self, ino: INodeNo) -> Option<FileAttr> {
        self.object_map.get(&ino).map(|inode| {
            // Force the root inode (1) and any YAFFS directory to act as a proper dir
            let is_dir = (inode.header.mode & S_IFDIR as u32) != 0 || ino.0 == 1;

            FileAttr {
                ino: inode.object_id,
                // Sanitize massive directory sizes caused by 0xFF erased blocks
                size: if is_dir {
                    4096
                } else {
                    inode.header.size as u64
                },
                blocks: if is_dir {
                    8
                } else {
                    (inode.header.size as u64 + 511) / 512
                },
                atime: SystemTime::UNIX_EPOCH + Duration::new(inode.header.atime as u64, 0),
                mtime: SystemTime::UNIX_EPOCH + Duration::new(inode.header.mtime as u64, 0),
                ctime: SystemTime::UNIX_EPOCH + Duration::new(inode.header.ctime as u64, 0),
                crtime: SystemTime::UNIX_EPOCH + Duration::new(inode.header.ctime as u64, 0),
                kind: if is_dir {
                    FileType::Directory
                } else {
                    FileType::RegularFile
                },
                perm: 0o777, // Force full access to bypass kernel security blocks for now
                nlink: if is_dir { 2 } else { 1 },
                uid: inode.header.uid,
                gid: inode.header.gid,
                rdev: 0,
                blksize: self.block_size as u32,
                flags: 0,
            }
        })
    }

    fn get_block_n(&self, inode: &INode, logical_block: u64) -> Option<Vec<u8>> {
        let leaf_index = logical_block & YAFFS_LEAF_MASK;
        let mut block_tree = inode.block_tree.clone();
        for h in (1..=inode.block_tree_height).rev() {
            let shift = (h - 1) as u64 * YAFFS_INTERNAL_BITS + YAFFS_LEAF_BITS;
            let tree_index = (logical_block >> shift) & YAFFS_INTERNAL_MASK;
            if let Tree::Internal(i) = block_tree {
                if i[tree_index as usize].is_none() {
                    return None;
                } else {
                    block_tree = i[tree_index as usize].clone().unwrap();
                }
            }
        }
        match block_tree {
            Tree::Internal(_) => None,
            Tree::Leaf(phys) => match bread(
                (self.mtd_page + self.mtd_extra) as u64,
                phys[leaf_index as usize] as u64,
                &self.file,
            ) {
                Ok(b) => Some(b),
                Err(_) => None,
            },
        }
    }

    fn add_data_block(&mut self, ino: INodeNo, logical_block: u32, physical_block: u32) {
        let inode = self.find_or_create_inode(ino);
        let leaf_index = (logical_block & YAFFS_LEAF_MASK as u32) as u64;
        let mut temp_index = logical_block >> YAFFS_LEAF_BITS;
        let mut required_height = 0;

        // 1. Calculate required height
        while temp_index > 0 {
            required_height += 1;
            temp_index >>= YAFFS_INTERNAL_BITS;
        }

        if inode.block_tree_height == 0 {
            if let Tree::Internal(_) = inode.block_tree {
                inode.block_tree = Tree::Leaf(Box::new([0; 16]));
            }
        }

        // 2. Grow the tree upwards if needed
        while inode.block_tree_height < required_height {
            let old_root = std::mem::replace(
                &mut inode.block_tree,
                Tree::Internal(Box::new([None, None, None, None, None, None, None, None])),
            );

            if let Tree::Internal(ref mut new_ptrs) = inode.block_tree {
                new_ptrs[0] = Some(old_root);
            }
            inode.block_tree_height += 1;
        }

        // 3. Traverse down to the leaf
        let mut current_node = &mut inode.block_tree;

        for h in (1..=inode.block_tree_height).rev() {
            let shift = (h - 1) as u64 * YAFFS_INTERNAL_BITS + YAFFS_LEAF_BITS;
            let branch_index = ((logical_block >> shift) & YAFFS_INTERNAL_MASK as u32) as u64;

            let next_node_is_leaf = h == 1;

            if let Tree::Internal(ptrs) = current_node {
                if ptrs[branch_index as usize].is_none() {
                    // Decide if next level is Internal or Leaf
                    ptrs[branch_index as usize] = Some(if next_node_is_leaf {
                        Tree::Leaf(Box::new([0; 16]))
                    } else {
                        Tree::Internal(Box::new([const { None }; 8]))
                    });
                }
                current_node = ptrs[branch_index as usize].as_mut().unwrap();
            } else {
                unreachable!()
            }
        }

        // 4. Finally, set the physical address in the leaf
        if let Tree::Leaf(physical_ptrs) = current_node {
            physical_ptrs[leaf_index as usize] = physical_block;
        } else {
            unreachable!()
        }
    }

    fn find_or_create_inode(&mut self, ino: INodeNo) -> &mut INode {
        self.object_map.entry(ino).or_insert_with(|| INode {
            object_id: ino,
            ..Default::default()
        })
    }
}

impl Filesystem for Yaffs2 {
    fn init(
        &mut self,
        _req: &fuser::Request,
        _config: &mut fuser::KernelConfig,
    ) -> std::io::Result<()> {
        let mut root_dir = INode::default();

        self.object_map = HashMap::new();
        let devsize = self.file.stream_len().unwrap();
        self.mtd_page = 2048;
        self.mtd_extra = 64;
        self.mtd_erase = 131072;
        self.block_size = self.mtd_page + self.mtd_extra;
        self.chunks_per_block = self.mtd_erase / self.mtd_page;

        let physical_erase_block_size = self.chunks_per_block * self.block_size;

        self.nblocks = devsize / physical_erase_block_size;
        self.nchunks = self.nblocks * self.chunks_per_block;

        root_dir.object_id = YAFFS_OBJECTID_ROOT;
        root_dir.header.mode = (S_IFDIR | 0o755) as u32;
        root_dir.header.object_type = ObjectType::YaffsObjectTypeDirectory as u32; // Add this!
        self.object_map.insert(root_dir.object_id, root_dir);

        for block in 0..self.nblocks {
            for chunk in 0..self.chunks_per_block {
                let buf = bread(
                    self.mtd_page + self.mtd_extra,
                    self.chunks_per_block * block + chunk,
                    &self.file,
                )
                .unwrap();

                let tags = unsafe {
                    std::ptr::read_unaligned(
                        buf.as_ptr().add(self.mtd_page as usize + 30) as *const Tags
                    )
                };

                let real_seq_number = tags.sequence_number;
                let is_header = (tags.object_id >> 28) > 0;
                let real_obj_id = tags.object_id & 0x0FFFFFFF;
                let real_chunk_id = tags.chunk_id & 0x0FFFFFFF;

                let object =
                    unsafe { std::ptr::read_unaligned(buf.as_ptr() as *const ObjectHeader) };

                if real_seq_number != !0 && is_header {
                    let parent_id = {
                        let inode = self.find_or_create_inode(INodeNo(real_obj_id as u64));
                        if real_seq_number > inode.sequence_number {
                            inode.header = object;
                            inode.sequence_number = real_seq_number;
                            Some(inode.header.parent_obj_id)
                        } else {
                            None
                        }
                    };

                    if let Some(pid) = parent_id {
                        if pid != 0 && pid != real_obj_id {
                            let parent = self.find_or_create_inode(INodeNo(pid as u64));
                            parent.children.push_front(INodeNo(real_obj_id as u64));
                        }
                    }
                } else if tags.chunk_id > 0 {
                    let addr = (block * self.chunks_per_block + chunk) as u32;
                    if real_chunk_id > 0 {
                        self.add_data_block(INodeNo(real_obj_id as u64), real_chunk_id - 1, addr);
                    }
                }
            }
        }

        Ok(())
    }

    fn lookup(
        &self,
        _req: &fuser::Request,
        parent: fuser::INodeNo,
        name: &std::ffi::OsStr,
        reply: fuser::ReplyEntry,
    ) {
        warn!("[Not Fully Implemented] lookup(parent: {parent:#x?}, name {name:?})");

        let target_name = match name.to_str() {
            Some(s) => s,
            None => {
                reply.error(Errno::ENOENT);
                return;
            }
        };

        match self.read_inode(parent) {
            Some(dir) => {
                match dir.children.iter().find(|c| {
                    let c = self.object_map.get(c).unwrap();
                    c.header
                        .name
                        .iter()
                        .map(|x| *x as char)
                        .collect::<String>()
                        .trim_matches('\0')
                        == target_name
                }) {
                    Some(inode) => match self.stat(*inode) {
                        Some(stat) => {
                            reply.entry(&Duration::new(10, 0), &stat, Generation(0));
                        }
                        None => {
                            reply.error(Errno::ENOENT);
                            return;
                        }
                    },
                    None => reply.error(Errno::ENOENT),
                }
            }
            None => {
                reply.error(Errno::ENOENT);
            }
        }
    }

    fn getattr(
        &self,
        _req: &fuser::Request,
        ino: fuser::INodeNo,
        fh: Option<fuser::FileHandle>,
        reply: fuser::ReplyAttr,
    ) {
        warn!("[Not Yet Implemented] getattr(ino: {ino:#x?}, fh: {fh:#x?})");

        match self.stat(ino) {
            Some(stat) => {
                info!("some stat: {stat:?}");
                reply.attr(&Duration::new(10, 0), &stat);
            }
            None => reply.error(Errno::ENOENT),
        }
    }

    fn open(
        &self,
        _req: &fuser::Request,
        ino: fuser::INodeNo,
        _flags: fuser::OpenFlags,
        reply: fuser::ReplyOpen,
    ) {
        match self.read_inode(ino) {
            Some(_) => {
                reply.opened(fuser::FileHandle(0), FopenFlags::empty());
            }
            None => reply.error(Errno::ENOENT),
        }
    }

    fn read(
        &self,
        _req: &fuser::Request,
        ino: fuser::INodeNo,
        _fh: fuser::FileHandle,
        offset: u64,
        size: u32,
        _flags: fuser::OpenFlags,
        _lock_owner: Option<fuser::LockOwner>,
        reply: fuser::ReplyData,
    ) {
        let inode = match self.read_inode(ino) {
            Some(node) => node,
            None => {
                reply.error(Errno::ENOENT);
                return;
            }
        };

        let file_size = inode.header.size as u64;

        // FIX 2: Safely handle FUSE reading at or past End-of-File
        if offset >= file_size {
            reply.data(&[]);
            return;
        }

        let size_to_read = (size as u64).min(file_size - offset);

        // FIX 3: Calculate using the logical payload size (2048), NOT block_size (2112)
        let payload_size = self.mtd_page;
        let start_block = offset / payload_size;
        let end_block = (offset + size_to_read - 1) / payload_size;

        let mut data = Vec::new();

        for logical_block in start_block..=end_block {
            let mut chunk_data = match self.get_block_n(inode, logical_block) {
                Some(b) => b,
                None => vec![0u8; payload_size as usize], // Fill sparse blocks with 0
            };

            // Strip out the 64-byte OOB area! FUSE only wants the clean payload.
            chunk_data.truncate(payload_size as usize);
            data.extend(chunk_data);
        }

        // Slice out the exact bytes the OS requested
        let start_in_vec = (offset % payload_size) as usize;
        let end_in_vec = start_in_vec + size_to_read as usize;

        reply.data(&data[start_in_vec..end_in_vec]);
    }

    fn release(
        &self,
        _req: &fuser::Request,
        _ino: fuser::INodeNo,
        _fh: fuser::FileHandle,
        _flags: fuser::OpenFlags,
        _lock_owner: Option<fuser::LockOwner>,
        _flush: bool,
        reply: fuser::ReplyEmpty,
    ) {
        reply.ok();
    }

    fn opendir(
        &self,
        _req: &fuser::Request,
        _ino: fuser::INodeNo,
        _flags: fuser::OpenFlags,
        reply: fuser::ReplyOpen,
    ) {
        reply.opened(fuser::FileHandle(0), fuser::FopenFlags::empty());
    }

    fn readdir(
        &self,
        _req: &fuser::Request,
        ino: fuser::INodeNo,
        _fh: fuser::FileHandle,
        offset: u64,
        mut reply: fuser::ReplyDirectory,
    ) {
        let dir = match self.read_inode(ino) {
            Some(dir) => dir,
            None => {
                reply.error(Errno::ENOENT);
                return;
            }
        };

        // 1. Manually inject the required "." and ".." FUSE entries
        let mut entries = vec![
            (ino, FileType::Directory, ".".to_string()),
            (
                INodeNo(if ino.0 == 1 {
                    1
                } else {
                    dir.header.parent_obj_id as u64
                }),
                FileType::Directory,
                "..".to_string(),
            ),
        ];

        // 2. Safely parse names, stopping at null bytes AND NAND erased markers (0xFF)
        let mut seen_names = std::collections::HashSet::new();

        for child_ino in &dir.children {
            if let Some(child_inode) = self.read_inode(*child_ino) {
                let name_bytes: Vec<u8> = child_inode
                    .header
                    .name
                    .iter()
                    .cloned()
                    .take_while(|&c| c != 0 && c != 0xFF) // Stop at 0xFF erased memory too!
                    .collect();

                let clean_name = String::from_utf8_lossy(&name_bytes).into_owned();

                // Prevent duplicate filenames from panicking FUSE
                if !clean_name.is_empty() && seen_names.insert(clean_name.clone()) {
                    let kind = if (child_inode.header.mode & S_IFDIR as u32) != 0 {
                        FileType::Directory
                    } else {
                        FileType::RegularFile
                    };
                    entries.push((*child_ino, kind, clean_name));
                }
            }
        }

        // 3. Yield to FUSE using proper chronological offsets
        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            // FUSE offsets must be exactly i + 1
            let buffer_full = reply.add(entry.0, (i + 1) as u64, entry.1, entry.2);
            if buffer_full {
                break;
            }
        }

        reply.ok();
    }

    fn releasedir(
        &self,
        _req: &fuser::Request,
        _ino: fuser::INodeNo,
        _fh: fuser::FileHandle,
        _flags: fuser::OpenFlags,
        reply: fuser::ReplyEmpty,
    ) {
        reply.ok();
    }

    fn statfs(&self, _req: &fuser::Request, _ino: fuser::INodeNo, reply: fuser::ReplyStatfs) {
        reply.statfs(
            self.nblocks,
            self.nblocks,
            self.nblocks,
            self.object_map.len() as u64,
            !0,
            self.mtd_page as u32,
            YAFFS_MAX_NAME_LENGTH as u32,
            self.mtd_page as u32,
        );
    }
}
