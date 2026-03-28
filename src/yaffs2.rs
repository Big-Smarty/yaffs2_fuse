pub mod config;
mod consts;
mod file_data;
mod inode;
mod object;
mod object_header;
mod object_type;
mod tags;
mod tree;
mod util;

use config::*;
use consts::*;
use inode::*;
use log::{error, info, trace};
use object_header::*;
use object_type::*;
use tags::*;
use util::*;

use std::{
    collections::LinkedList,
    io,
    time::{Duration, SystemTime},
};

use dashmap::DashMap;
use fuser::{Errno, FileAttr, FileType, Filesystem, FopenFlags, Generation, INodeNo};
use memmap2::Mmap;

use consts::*;

use crate::yaffs2::{file_data::FileData, object::Object, tree::Tree};

pub struct Yaffs2 {
    image: Mmap,
    objects: DashMap<INodeNo, INode>,
    hierarchy: DashMap<INodeNo, Vec<INodeNo>>,
    config: Config,
}

impl Yaffs2 {
    pub fn new(
        image: Mmap,
        mtd_page_size: usize,
        mtd_extra_size: usize,
        mtd_erase: usize,
        offset: usize,
    ) -> io::Result<Self> {
        let config = Config::new(
            image.len(),
            mtd_page_size,
            mtd_extra_size,
            mtd_erase,
            offset,
        );
        Ok(Self {
            image,
            objects: DashMap::new(),
            hierarchy: DashMap::new(),
            config,
        })
    }

    fn stat(&self, ino: INodeNo) -> Option<FileAttr> {
        match self.objects.get(&ino) {
            Some(inode) => {
                let size = match inode.header.object_type {
                    FileType::Directory => 0,
                    _ => inode.data.as_ref().map(|d| d.len()).unwrap_or(0) as u64,
                };

                Some(FileAttr {
                    ino,
                    size,
                    blocks: (size + 511) / 512,
                    atime: SystemTime::UNIX_EPOCH + Duration::new(inode.header.atime as u64, 0),
                    ctime: SystemTime::UNIX_EPOCH + Duration::new(inode.header.ctime as u64, 0),
                    crtime: SystemTime::UNIX_EPOCH + Duration::new(inode.header.ctime as u64, 0),
                    mtime: SystemTime::UNIX_EPOCH + Duration::new(inode.header.mtime as u64, 0),
                    kind: inode.header.object_type,
                    perm: (inode.header.mode & 0o777) as u16,
                    nlink: if inode.header.object_type == FileType::Directory {
                        2
                    } else {
                        1
                    },
                    uid: inode.header.uid,
                    gid: inode.header.gid,
                    rdev: inode.header.rdev,
                    blksize: self.config.page_size as u32,
                    flags: 0,
                })
            }
            None => None,
        }
    }

    fn get_block_n(&self, ino: INodeNo, logical_block: u64) -> Option<Vec<u8>> {
        todo!()
    }

    fn find_or_create_inode(&mut self, ino: INodeNo) -> &mut INode {
        todo!()
    }

    fn add_data_block(&mut self, ino: INodeNo, logical_block: u64, physical_block: u64) {
        todo!()
    }

    fn cread(&self, chunk: usize) -> &[u8] {
        let physical = chunk * (self.config.page_size + self.config.extra_size);
        let out = &self.image[physical..physical + self.config.page_size + self.config.extra_size];
        out
    }
}

impl Filesystem for Yaffs2 {
    fn init(&mut self, _req: &fuser::Request, _config: &mut fuser::KernelConfig) -> io::Result<()> {
        info!("init");
        let mut root_dir = INode::default();
        root_dir.object_id = YAFFS_OBJECTID_ROOT;
        root_dir.header.mode = (S_IFDIR | 0o755) as u32;
        root_dir.header.object_type = FileType::Directory;
        root_dir.header.parent_id = YAFFS_OBJECTID_ROOT;
        self.objects.insert(YAFFS_OBJECTID_ROOT, root_dir);
        self.hierarchy.insert(YAFFS_OBJECTID_ROOT, Vec::new());

        let mut files: DashMap<u64, FileData> = DashMap::new();

        for block in 0..self.config.nblocks {
            for chunk in 0..self.config.chunks_per_block {
                let chunk_idx = block * self.config.chunks_per_block + chunk;
                let chunk_data = self.cread(chunk_idx);

                // Try reading tags from the OOB area
                let tags_start = self.config.page_size + self.config.tags_offset;
                let tags_end = tags_start + std::mem::size_of::<PackedTags>();

                if tags_end <= chunk_data.len() {
                    let tags_data = &chunk_data[tags_start..tags_end];
                    let tags: PackedTags = unsafe {
                        std::ptr::read_unaligned(tags_data.as_ptr() as *const PackedTags)
                    };
                    let tags: Tags = tags.into();

                    // Validate the tags - Yaffs2 valid chunk criteria:
                    // 1. sequence_number > 0 and < some reasonable max
                    // 2. object_id > 0 and < YAFFS_MAX_OBJECTS (typically 100000)
                    // 3. chunk_id should be 0 for headers or within file size for data
                    // 4. num_data_bytes <= page_size
                    if tags.sequence_number != 0
                        && tags.sequence_number != 0xFFFFFFFF
                        && tags.object_id != 0
                        && tags.object_id != 0xFFFFFFFF
                        && tags.num_data_bytes <= self.config.page_size as u64
                    {
                        trace!("Valid tag at block {}, chunk {}: {:?}", block, chunk, tags);

                        let mut file = files
                            .entry(tags.object_id)
                            .or_insert_with(|| FileData::new(INodeNo(tags.object_id)));

                        if tags.is_header {
                            trace!("Found object header for object_id: {}", tags.object_id);
                            let header_data = &chunk_data[0..std::mem::size_of::<ObjectHeader>()];
                            let header: ObjectHeader = unsafe {
                                std::ptr::read_unaligned(header_data.as_ptr() as *const ObjectHeader)
                            };
                            let mut header: Header = header.into();
                            trace!("{header:?}");

                            if tags.object_id == YAFFS_OBJECTID_ROOT.0 {
                                header.parent_id = YAFFS_OBJECTID_ROOT;
                                trace!("Fixed root directory parent_id to {}", YAFFS_OBJECTID_ROOT);
                            }

                            // Get or create inode
                            let inode = if let Some(existing) =
                                self.objects.get(&INodeNo(tags.object_id))
                            {
                                existing.clone()
                            } else {
                                let new_inode = INode {
                                    object_id: INodeNo(tags.object_id),
                                    header: header.clone(),
                                    sequence_number: tags.sequence_number,
                                    data: None,
                                };
                                self.objects
                                    .insert(INodeNo(tags.object_id), new_inode.clone());
                                new_inode
                            };

                            // Only update if this is a newer version
                            if tags.sequence_number > inode.sequence_number {
                                let mut mutable_inode =
                                    self.objects.get_mut(&INodeNo(tags.object_id)).unwrap();
                                mutable_inode.header = header.clone();
                                mutable_inode.sequence_number = tags.sequence_number;
                            }

                            // Build hierarchy - add child to parent's list
                            let parent_id = header.parent_id;
                            let child_id = INodeNo(tags.object_id);

                            // Skip special objects
                            if child_id != YAFFS_OBJECTID_ROOT
                                && child_id != YAFFS_OBJECTID_UNLINKED
                                && child_id != YAFFS_OBJECTID_DELETED
                            {
                                // Ensure parent exists in hierarchy
                                let mut children =
                                    self.hierarchy.entry(parent_id).or_insert_with(Vec::new);

                                // Avoid duplicates
                                if !children.contains(&child_id) {
                                    children.push(child_id);
                                    trace!("Added child {} to parent {}", child_id, parent_id);
                                }
                            }

                            file.header = Some(header.clone());
                            file.size = header.size as usize;
                        } else {
                            // This is a data chunk
                            trace!(
                                "Found data chunk for object_id: {}, chunk_id: {}",
                                tags.object_id, tags.chunk_id
                            );

                            let data_range = 0..(tags.num_data_bytes as usize);
                            let data_buf = chunk_data[data_range].to_vec();

                            file.add_chunk(tags.chunk_id as usize, &data_buf);
                        }
                    } else {
                        // Optional: print invalid tags only if you want to debug
                        // println!("Invalid tag at block {}, chunk {}: {:?}", block, chunk, tags);
                    }
                }
            }
        }

        // Post-scan: reconstruct all files now that headers and chunks are fully collected
        for entry in files.iter() {
            let file = entry.value();
            if let Some(data) = file.reconstruct(self.config.page_size) {
                if let Some(mut inode) = self.objects.get_mut(&file.object_id) {
                    inode.data = Some(data);
                    info!(
                        "Reconstructed file {} ({} bytes)",
                        file.object_id,
                        inode.data.as_ref().unwrap().len()
                    );
                }
            }
        }

        // At the end of init, after processing all chunks
        info!("=== File Completion Statistics ===");
        let mut files_with_data = 0;
        let mut total_bytes = 0;
        for entry in self.objects.iter() {
            if let Some(data) = &entry.data {
                files_with_data += 1;
                total_bytes += data.len();
                info!(
                    "File: {} (id={}) - {} bytes",
                    entry.header.name,
                    entry.object_id,
                    data.len()
                );
            }
        }
        info!(
            "Total files with data: {}, total bytes: {}",
            files_with_data, total_bytes
        );

        Ok(())
    }

    fn lookup(
        &self,
        _req: &fuser::Request,
        parent: INodeNo,
        name: &std::ffi::OsStr,
        reply: fuser::ReplyEntry,
    ) {
        let name_str = name.to_str().unwrap();
        info!("lookup: parent={}, name={}", parent, name_str);

        if name_str == "." {
            if let Some(attr) = self.stat(parent) {
                reply.entry(&Duration::new(1, 0), &attr, Generation(0));
                return;
            }
        }

        // Handle ".." (parent directory)
        if name_str == ".." {
            if let Some(inode) = self.objects.get(&parent) {
                let parent_id = inode.value().header.parent_id;
                info!("lookup: .. parent_id={}", parent_id);
                if let Some(attr) = self.stat(parent_id) {
                    reply.entry(&Duration::new(1, 0), &attr, Generation(0));
                    return;
                }
            }
        }

        match self.hierarchy.get(&parent) {
            Some(children) => {
                info!(
                    "lookup: found {} children for parent {}",
                    children.value().len(),
                    parent
                );
                for child_ino in children.value() {
                    if let Some(child) = self.objects.get(child_ino) {
                        let child_name = &child.value().header.name;
                        info!(
                            "lookup: checking child {}: name='{}' against '{}'",
                            child_ino, child_name, name_str
                        );
                        if child_name == name_str {
                            if let Some(attr) = self.stat(*child_ino) {
                                info!("lookup: found match! inode={}", child_ino);
                                reply.entry(&Duration::new(1, 0), &attr, Generation(0));
                                return;
                            }
                        }
                    }
                }
                info!("lookup: no match found for '{}'", name_str);
                reply.error(Errno::ENOENT);
            }
            None => {
                info!("lookup: parent {} has no children in hierarchy", parent);
                reply.error(Errno::ENOENT);
            }
        }
    }

    fn getattr(
        &self,
        _req: &fuser::Request,
        ino: INodeNo,
        _fh: Option<fuser::FileHandle>,
        reply: fuser::ReplyAttr,
    ) {
        match self.stat(ino) {
            Some(attr) => reply.attr(&Duration::new(1, 0), &attr),
            None => reply.error(Errno::ENOENT),
        }
    }

    fn open(
        &self,
        _req: &fuser::Request,
        ino: INodeNo,
        _flags: fuser::OpenFlags,
        reply: fuser::ReplyOpen,
    ) {
        if self.objects.contains_key(&ino) {
            reply.opened(fuser::FileHandle(0), FopenFlags::empty());
        } else {
            reply.error(Errno::ENOENT);
        }
    }

    fn read(
        &self,
        _req: &fuser::Request,
        ino: INodeNo,
        _fh: fuser::FileHandle,
        offset: u64,
        size: u32,
        _flags: fuser::OpenFlags,
        _lock_owner: Option<fuser::LockOwner>,
        reply: fuser::ReplyData,
    ) {
        match self.objects.get(&ino) {
            Some(inode) => {
                info!(
                    "read: inode={}, offset={}, size={}, has_data={}, data_len={}",
                    ino,
                    offset,
                    size,
                    inode.data.is_some(),
                    inode.data.as_ref().map(|d| d.len()).unwrap_or(0)
                );

                if let Some(data) = &inode.data {
                    let start = offset as usize;
                    let end = std::cmp::min(start + size as usize, data.len());

                    if start >= data.len() {
                        reply.data(&[]);
                    } else {
                        reply.data(&data[start..end]);
                    }
                } else {
                    // Directory or file with no data
                    reply.data(&[]);
                }
            }
            None => reply.error(Errno::ENOENT),
        }
    }

    fn release(
        &self,
        _req: &fuser::Request,
        _ino: INodeNo,
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
        _ino: INodeNo,
        _flags: fuser::OpenFlags,
        reply: fuser::ReplyOpen,
    ) {
        reply.opened(fuser::FileHandle(0), fuser::FopenFlags::empty());
    }

    fn readdir(
        &self,
        _req: &fuser::Request,
        ino: INodeNo,
        _fh: fuser::FileHandle,
        offset: u64,
        mut reply: fuser::ReplyDirectory,
    ) {
        // Get the inode to verify it's a directory
        let inode = match self.objects.get(&ino) {
            Some(inode) => inode,
            None => {
                error!("readdir: inode {} not found", ino);
                reply.error(Errno::ENOENT);
                return;
            }
        };

        if inode.header.object_type != FileType::Directory {
            error!("readdir: inode {} is not a directory", ino);
            reply.error(Errno::ENOTDIR);
            return;
        }

        // Get children from hierarchy (or empty vec if none)
        let children = self
            .hierarchy
            .get(&ino)
            .map(|c| c.value().clone())
            .unwrap_or_default();

        trace!(
            "readdir: inode={}, children count={}, offset={}",
            ino,
            children.len(),
            offset
        );

        // Build list of all entries: . and .. plus all children
        let mut entries = Vec::new();

        // Add "." entry
        entries.push((ino, 1, FileType::Directory, ".".to_string()));

        // Add ".." entry
        let parent_id = if ino == YAFFS_OBJECTID_ROOT {
            YAFFS_OBJECTID_ROOT
        } else {
            inode.header.parent_id
        };
        entries.push((parent_id, 2, FileType::Directory, "..".to_string()));

        // Add all children
        for (i, child_ino) in children.iter().enumerate() {
            if let Some(child) = self.objects.get(child_ino) {
                let child_inode = child.value();
                let position = (i + 3) as i64; // positions: 1=".", 2="..", then 3,4,5...
                entries.push((
                    *child_ino,
                    position,
                    child_inode.header.object_type,
                    child_inode.header.name.clone(),
                ));
            }
        }

        // Start from the offset
        for (ino, position, kind, name) in entries.into_iter().skip(offset as usize) {
            if reply.add(ino, position as u64, kind, &name) {
                // Buffer is full, stop adding
                break;
            }
        }

        reply.ok();
    }

    fn statfs(&self, _req: &fuser::Request, _ino: INodeNo, reply: fuser::ReplyStatfs) {
        reply.statfs(
            self.config.nblocks as u64,
            self.config.nblocks as u64,
            self.config.nblocks as u64,
            self.objects.len() as u64,
            !0,
            self.config.page_size as u32,
            YAFFS_MAX_NAME_LENGTH as u32,
            self.config.page_size as u32,
        );
    }
}
