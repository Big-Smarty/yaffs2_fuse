use fuser::INodeNo;

use crate::yaffs2::object_header::Header;

#[derive(Clone, Debug)]
pub struct INode {
    pub header: Header,
    pub object_id: INodeNo,
    pub sequence_number: u64,
    pub data: Option<Vec<u8>>,
}

impl Default for INode {
    fn default() -> Self {
        Self {
            header: Default::default(),
            object_id: INodeNo(0),
            sequence_number: Default::default(),
            data: None,
        }
    }
}
