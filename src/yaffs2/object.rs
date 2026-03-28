use crate::yaffs2::{object_header::Header, tags::Tags};

#[derive(Clone, Debug)]
pub struct Object {
    pub header: Header,
    pub tags: Tags,
}
