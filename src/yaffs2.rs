pub mod util;

use std::{
    collections::{HashMap, LinkedList},
    fs::File,
    io::{self, Seek},
    path::PathBuf,
    time::{Duration, SystemTime},
};

// TODO: make buffer lifetimes dependant on Yaffs2Info Lifetime

use fuser::{Errno, FileAttr, FileType, Filesystem, FopenFlags, Generation, INodeNo};
use log::{info, warn};

use crate::yaffs2::util::bread;

pub const YAFFS_MAGIC: u64 = 0x5941FF53;
pub const YAFFS_MAX_NAME_LENGTH: u64 = 255;
pub const YAFFS_MAX_ALIAS_LENGTH: u64 = 159;
pub const YAFFS_OBJECTID_ROOT: INodeNo = INodeNo(1);

pub const YAFFS_LEAF_BITS: u64 = 4;
pub const YAFFS_LEAF_MASK: u64 = 0xf;

pub const YAFFS_INTERNAL_BITS: u64 = 3;
pub const YAFFS_INTERNAL_MASK: u64 = 0x7;

pub const S_IFDIR: u64 = 16384;

#[repr(u32)]
pub enum ObjectType {
    YaffsObjectTypeUnknown,
    YaffsObjectTypeFile,
    YaffsObjectTypeSymlink,
    YaffsObjectTypeDirectory,
    YaffsObjectTypeHardlink,
    YaffsObjectTypeSpecial,
}

#[repr(C)]
#[repr(packed)]
#[derive(Copy, Clone, Debug, Default)]
pub struct Yaffs2Tags {
    pub sequence_number: u32,
    pub object_id: u32,
    pub chunk_id: u32,
    pub num_data_bytes: u32,
}

/*
struct yaffs2_object_header {
    __le32 object_type;
    __le32 parent_object_id;
    __le16 sum_obsolete;
    char name[YAFFS_MAX_NAME_LENGTH + 1];
    __le32 mode;
    __le32 uid;
    __le32 gid;
    __le32 atime;
    __le32 mtime;
    __le32 ctime;
    __le32 size;
    __le32 equiv_object_id;
    char alias[YAFFS_MAX_NAME_LENGTH + 1];
    __le32 rdev;
    __le32 reserved[6];
    __le32 inband_shadows_object;
    __le32 inband_is_shrink;
    __le32 reserved2[2];
    __le32 shadows_object;
    __le32 is_shrink;
};

*/

#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct Yaffs2ObjectHeader {
    pub object_type: u32,
    pub parent_obj_id: u32,
    pub sum_obsolete: u16,
    pub name: [u8; YAFFS_MAX_NAME_LENGTH as usize + 1],
    pub _padding: u16, // <-- ADD THIS TO CATCH THE C-COMPILER PADDING
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub atime: u32,
    pub mtime: u32,
    pub ctime: u32,
    pub size: u32,
    pub equiv_obj_id: u32,
    pub alias: [u8; YAFFS_MAX_ALIAS_LENGTH as usize + 1],
    // Note: You might need to check if alias requires padding too!
    // alias is 160 bytes. 268 + 24 (mode->size) + 4 (equiv) + 160 = 456.
    // 456 is divisible by 4, so no padding should be needed here.
    pub rdev: u32,
    pub reserved: [u32; 6],
    pub inband_shadow_objects: u32,
    pub inband_is_shrink: u32,
    pub reserved2: [u32; 2],
    pub shadows_object: u32,
    pub is_shrink: u32,
}

impl Default for Yaffs2ObjectHeader {
    fn default() -> Self {
        Self {
            object_type: Default::default(),
            parent_obj_id: Default::default(),
            sum_obsolete: Default::default(),
            name: [0; YAFFS_MAX_NAME_LENGTH as usize + 1],
            mode: Default::default(),
            uid: Default::default(),
            gid: Default::default(),
            atime: Default::default(),
            mtime: Default::default(),
            ctime: Default::default(),
            size: Default::default(),
            equiv_obj_id: Default::default(),
            alias: [0; YAFFS_MAX_ALIAS_LENGTH as usize + 1],
            rdev: Default::default(),
            reserved: Default::default(),
            inband_shadow_objects: Default::default(),
            reserved2: Default::default(),
            shadows_object: Default::default(),
            is_shrink: Default::default(),
            _padding: 0,
            inband_is_shrink: Default::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Yaffs2Tree {
    Internal(Box<[Option<Yaffs2Tree>; 8]>),
    Leaf(Box<[u32; 16]>),
}

impl Default for Yaffs2Tree {
    fn default() -> Self {
        Self::Internal(Box::new([const { None }; 8]))
    }
}

#[derive(Clone)]
pub struct Yaffs2Inode {
    pub header: Yaffs2ObjectHeader,
    pub object_id: INodeNo,
    pub sequence_number: u32,
    pub children: LinkedList<INodeNo>,
    pub block_tree: Yaffs2Tree,
    pub block_tree_height: u64,
}

impl PartialEq for Yaffs2Inode {
    fn eq(&self, other: &Self) -> bool {
        self.object_id == other.object_id
    }
}

impl Default for Yaffs2Inode {
    fn default() -> Self {
        Self {
            header: Default::default(),
            object_id: INodeNo(0),
            sequence_number: Default::default(),
            children: Default::default(),
            block_tree: Default::default(),
            block_tree_height: Default::default(),
        }
    }
}

unsafe impl Send for Yaffs2Inode {}
unsafe impl Sync for Yaffs2Inode {}

pub struct Yaffs2Info {
    pub file: File,
    pub mtd_page: u64,
    pub mtd_extra: u64,
    pub mtd_erase: u64,
    pub chunks_per_block: u64,
    pub nblocks: u64,
    pub nchunks: u64,
    pub block_size: u64,
    pub object_map: HashMap<INodeNo, Yaffs2Inode>,
    pub buffers: Vec<Vec<u8>>,
}

pub struct Yaffs2 {
    pub info: Yaffs2Info,
}

impl Yaffs2 {
    pub fn new(path: PathBuf) -> io::Result<Self> {
        match File::open(path) {
            Ok(file) => Ok(Self {
                info: Yaffs2Info {
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
                },
            }),
            Err(e) => Err(e),
        }
    }

    fn read_inode(&self, ino: INodeNo) -> Option<&Yaffs2Inode> {
        self.info.object_map.get(&ino)
    }

    fn stat(&self, ino: INodeNo) -> Option<FileAttr> {
        self.info.object_map.get(&ino).map(|inode| {
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
                blksize: self.info.block_size as u32,
                flags: 0,
            }
        })
    }

    fn get_block_n(&self, inode: &Yaffs2Inode, logical_block: u64) -> Option<Vec<u8>> {
        let leaf_index = logical_block & YAFFS_LEAF_MASK;
        let mut block_tree = inode.block_tree.clone();
        for h in (1..=inode.block_tree_height).rev() {
            let shift = (h - 1) as u64 * YAFFS_INTERNAL_BITS + YAFFS_LEAF_BITS;
            let tree_index = (logical_block >> shift) & YAFFS_INTERNAL_MASK;
            if let Yaffs2Tree::Internal(i) = block_tree {
                if i[tree_index as usize].is_none() {
                    return None;
                } else {
                    block_tree = i[tree_index as usize].clone().unwrap();
                }
            }
        }
        match block_tree {
            Yaffs2Tree::Internal(_) => None,
            Yaffs2Tree::Leaf(phys) => match bread(
                (self.info.mtd_page + self.info.mtd_extra) as u64,
                phys[leaf_index as usize] as u64,
                &self.info.file,
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
            if let Yaffs2Tree::Internal(_) = inode.block_tree {
                inode.block_tree = Yaffs2Tree::Leaf(Box::new([0; 16]));
            }
        }

        // 2. Grow the tree upwards if needed
        while inode.block_tree_height < required_height {
            let old_root = std::mem::replace(
                &mut inode.block_tree,
                Yaffs2Tree::Internal(Box::new([None, None, None, None, None, None, None, None])),
            );

            if let Yaffs2Tree::Internal(ref mut new_ptrs) = inode.block_tree {
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

            if let Yaffs2Tree::Internal(ptrs) = current_node {
                if ptrs[branch_index as usize].is_none() {
                    // Decide if next level is Internal or Leaf
                    ptrs[branch_index as usize] = Some(if next_node_is_leaf {
                        Yaffs2Tree::Leaf(Box::new([0; 16]))
                    } else {
                        Yaffs2Tree::Internal(Box::new([const { None }; 8]))
                    });
                }
                current_node = ptrs[branch_index as usize].as_mut().unwrap();
            } else {
                unreachable!()
            }
        }

        // 4. Finally, set the physical address in the leaf
        if let Yaffs2Tree::Leaf(physical_ptrs) = current_node {
            physical_ptrs[leaf_index as usize] = physical_block;
        } else {
            unreachable!()
        }
    }

    fn find_or_create_inode(&mut self, ino: INodeNo) -> &mut Yaffs2Inode {
        self.info
            .object_map
            .entry(ino)
            .or_insert_with(|| Yaffs2Inode {
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
        let mut root_dir = Yaffs2Inode::default();

        self.info.object_map = HashMap::new();
        let devsize = self.info.file.stream_len().unwrap();
        self.info.mtd_page = 2048;
        self.info.mtd_extra = 64;
        self.info.mtd_erase = 131072;
        self.info.block_size = self.info.mtd_page + self.info.mtd_extra;
        self.info.chunks_per_block = self.info.mtd_erase / self.info.mtd_page;

        let physical_erase_block_size = self.info.chunks_per_block * self.info.block_size;

        self.info.nblocks = devsize / physical_erase_block_size;
        self.info.nchunks = self.info.nblocks * self.info.chunks_per_block;

        root_dir.object_id = YAFFS_OBJECTID_ROOT;
        root_dir.header.mode = (S_IFDIR | 0o755) as u32;
        root_dir.header.object_type = ObjectType::YaffsObjectTypeDirectory as u32; // Add this!
        self.info.object_map.insert(root_dir.object_id, root_dir);

        for block in 0..self.info.nblocks {
            for chunk in 0..self.info.chunks_per_block {
                let buf = bread(
                    self.info.mtd_page + self.info.mtd_extra,
                    self.info.chunks_per_block * block + chunk,
                    &self.info.file,
                )
                .unwrap();

                let tags = unsafe {
                    std::ptr::read_unaligned(
                        buf.as_ptr().add(self.info.mtd_page as usize + 30) as *const Yaffs2Tags
                    )
                };

                let real_seq_number = tags.sequence_number;
                let is_header = (tags.object_id >> 28) > 0;
                let real_obj_id = tags.object_id & 0x0FFFFFFF;
                let real_chunk_id = tags.chunk_id & 0x0FFFFFFF;

                let object =
                    unsafe { std::ptr::read_unaligned(buf.as_ptr() as *const Yaffs2ObjectHeader) };

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
                    let addr = (block * self.info.chunks_per_block + chunk) as u32;
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
                    let c = self.info.object_map.get(c).unwrap();
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
        let payload_size = self.info.mtd_page;
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
        reply.statfs(0, 0, 0, 0, 0, 512, 255, 0);
    }
}
