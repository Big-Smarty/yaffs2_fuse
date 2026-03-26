use crate::yaffs2::consts::*;

#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct ObjectHeader {
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

impl Default for ObjectHeader {
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
