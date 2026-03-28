use fuser::{FileType, INodeNo};

use crate::yaffs2::{consts::*, object_type::ObjectType};

#[derive(Clone, Debug)]
pub struct Header {
    pub object_type: FileType,
    pub parent_id: INodeNo,
    pub name: String,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub atime: u32,
    pub mtime: u32,
    pub ctime: u32,
    pub size: u32,
    pub rdev: u32,
}

impl Default for Header {
    fn default() -> Self {
        Self {
            object_type: FileType::Directory,
            parent_id: YAFFS_OBJECTID_ROOT,
            name: Default::default(),
            mode: Default::default(),
            uid: Default::default(),
            gid: Default::default(),
            atime: Default::default(),
            mtime: Default::default(),
            ctime: Default::default(),
            size: Default::default(),
            rdev: Default::default(),
        }
    }
}

impl From<ObjectHeader> for Header {
    fn from(value: ObjectHeader) -> Self {
        Self {
            object_type: if value.object_type == ObjectType::YaffsObjectTypeDirectory as u32 {
                FileType::Directory
            } else {
                FileType::RegularFile
            },
            parent_id: INodeNo(value.parent_obj_id as u64),
            name: value
                .name
                .iter()
                .map(|x| *x as char)
                .collect::<String>()
                .trim_matches('\0')
                .to_string(),
            mode: (S_IFDIR as u32 | 0o777),
            uid: value.uid,
            gid: value.gid,
            atime: value.atime,
            mtime: value.mtime,
            ctime: value.ctime,
            size: value.size,
            rdev: value.rdev,
        }
    }
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct ObjectHeader {
    pub object_type: u32,
    pub parent_obj_id: u32,
    pub sum_obsolete: u16,
    pub name: [u8; YAFFS_MAX_NAME_LENGTH as usize + 1],
    pub _padding: u16,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub atime: u32,
    pub mtime: u32,
    pub ctime: u32,
    pub size: u32,
    pub equiv_obj_id: u32,
    pub alias: [u8; YAFFS_MAX_ALIAS_LENGTH as usize + 1],
    pub rdev: u32,
    pub reserved: [u32; 6],
    pub inband_shadow_objects: u32,
    pub inband_is_shrink: u32,
    pub reserved2: [u32; 2],
    pub shadows_object: u32,
    pub is_shrink: u32,
}

impl Default for ObjectHeader {
    fn default() -> Self {
        Self {
            object_type: Default::default(),
            parent_obj_id: Default::default(),
            sum_obsolete: Default::default(),
            name: [0; YAFFS_MAX_NAME_LENGTH as usize + 1],
            _padding: Default::default(),
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
            inband_is_shrink: Default::default(),
            reserved2: Default::default(),
            shadows_object: Default::default(),
            is_shrink: Default::default(),
        }
    }
}
