#[repr(u32)]
pub enum ObjectType {
    YaffsObjectTypeUnknown,
    YaffsObjectTypeFile,
    YaffsObjectTypeSymlink,
    YaffsObjectTypeDirectory,
    YaffsObjectTypeHardlink,
    YaffsObjectTypeSpecial,
}
