use fuser::INodeNo;

pub const YAFFS_MAX_NAME_LENGTH: u64 = 255;
pub const YAFFS_MAX_ALIAS_LENGTH: u64 = 159;
pub const YAFFS_OBJECTID_ROOT: INodeNo = INodeNo(1);
pub const YAFFS_OBJECTID_UNLINKED: INodeNo = INodeNo(3);
pub const YAFFS_OBJECTID_DELETED: INodeNo = INodeNo(4);

pub const S_IFDIR: u64 = 16384;
