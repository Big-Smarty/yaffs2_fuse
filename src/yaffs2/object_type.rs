use fuser::FileType;

#[repr(u32)]
pub enum ObjectType {
    YaffsObjectTypeDirectory = 3,
}

impl Into<FileType> for ObjectType {
    fn into(self) -> FileType {
        match self {
            ObjectType::YaffsObjectTypeDirectory => FileType::Directory,
        }
    }
}
