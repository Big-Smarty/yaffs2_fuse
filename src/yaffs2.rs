use std::{
    collections::HashMap,
    fs::File,
    io,
    path::PathBuf,
    time::{Duration, SystemTime},
};

use fuser::{FileAttr, FileType, INodeNo};

use crate::yaffs2::{inode::INode, tree::Tree, util::bread};

mod consts;
mod inode;
mod object_header;
mod object_type;
mod tags;
mod tree;
mod util;

use consts::*;

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

    fn stat(&self, ino: INodeNo) -> Option<FileAttr> {
        match self.object_map.get(&ino) {
            Some(inode) => {
                let is_dir = (inode.header.mode & S_IFDIR as u32) != 0 || ino.0 == 1;

                Some(FileAttr {
                    ino,
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
                    perm: 0o777,
                    nlink: if is_dir { 2 } else { 1 },
                    uid: inode.header.uid,
                    gid: inode.header.gid,
                    rdev: 0,
                    blksize: self.block_size as u32,
                    flags: 0,
                })
            }
            None => None,
        }
    }

    fn get_block_n(&self, ino: INodeNo, logical_block: u64) -> Option<Vec<u8>> {
        match self.object_map.get(&ino) {
            Some(inode) => {
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
            None => None,
        }
    }

    fn find_or_create_inode(&mut self, ino: INodeNo) -> &mut INode {
        self.object_map.entry(ino).or_insert_with(|| INode {
            object_id: ino,
            ..Default::default()
        })
    }

    fn add_data_block(&mut self, ino: INodeNo, logical_block: u64, physical_block: u64) {
        let inode = self.find_or_create_inode(ino);

        let leaf_index = logical_block & YAFFS_LEAF_MASK;
        let mut temp_index = logical_block >> YAFFS_LEAF_BITS;
        let mut required_height = 0;

        while temp_index > 0 {
            required_height += 1;
            temp_index >>= YAFFS_INTERNAL_BITS;
        }

        if inode.block_tree_height == 0 {
            if let Tree::Internal(_) = inode.block_tree {
                inode.block_tree = Tree::Leaf(Box::new([0; 16]));
            }
        }

        while inode.block_tree_height < required_height {
            let old_root = std::mem::replace(
                &mut inode.block_tree,
                Tree::Internal(Box::new([const { None }; 8])),
            );

            if let Tree::Internal(ref mut new_ptrs) = inode.block_tree {
                new_ptrs[0] = Some(old_root);
            }
            inode.block_tree_height += 1;
        }

        let mut current_node = &mut inode.block_tree;

        for h in (1..=inode.block_tree_height).rev() {
            let tree_index = (logical_block >> ((h - 1) * YAFFS_INTERNAL_BITS + YAFFS_LEAF_BITS))
                & YAFFS_INTERNAL_MASK;

            if let Tree::Internal(ptrs) = current_node {
                match ptrs[tree_index as usize] {
                    Some(ref mut node) => current_node = node,
                    None => todo!(),
                }
            }
        }
    }
}
