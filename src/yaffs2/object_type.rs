use fuser::FileType;

#[repr(u32)]
pub enum ObjectType {
    YaffsObjectTypeUnknown,
    YaffsObjectTypeFile,
    YaffsObjectTypeSymlink,
    YaffsObjectTypeDirectory,
    YaffsObjectTypeHardlink,
    YaffsObjectTypeSpecial,
}

impl Into<FileType> for ObjectType {
    fn into(self) -> FileType {
        match self {
            ObjectType::YaffsObjectTypeUnknown => FileType::RegularFile,
            ObjectType::YaffsObjectTypeFile => FileType::RegularFile,
            ObjectType::YaffsObjectTypeSymlink => FileType::Symlink,
            ObjectType::YaffsObjectTypeDirectory => FileType::Directory,
            ObjectType::YaffsObjectTypeHardlink => FileType::RegularFile,
            ObjectType::YaffsObjectTypeSpecial => FileType::RegularFile,
        }
    }
}
