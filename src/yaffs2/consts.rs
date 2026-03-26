use fuser::INodeNo;

pub const YAFFS_MAGIC: u64 = 0x5941FF53;
pub const YAFFS_MAX_NAME_LENGTH: u64 = 255;
pub const YAFFS_MAX_ALIAS_LENGTH: u64 = 159;
pub const YAFFS_OBJECTID_ROOT: INodeNo = INodeNo(1);

pub const YAFFS_LEAF_BITS: u64 = 4;
pub const YAFFS_LEAF_MASK: u64 = 0xf;

pub const YAFFS_INTERNAL_BITS: u64 = 3;
pub const YAFFS_INTERNAL_MASK: u64 = 0x7;

pub const S_IFDIR: u64 = 16384;
