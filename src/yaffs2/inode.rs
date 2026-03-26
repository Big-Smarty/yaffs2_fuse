use std::collections::LinkedList;

use fuser::INodeNo;

use crate::yaffs2::{object_header::ObjectHeader, tree::Tree};

#[derive(Clone)]
pub struct INode {
    pub header: ObjectHeader,
    pub object_id: INodeNo,
    pub sequence_number: u32,
    pub children: LinkedList<INodeNo>,
    pub block_tree: Tree,
    pub block_tree_height: u64,
}

impl PartialEq for INode {
    fn eq(&self, other: &Self) -> bool {
        self.object_id == other.object_id
    }
}

impl Default for INode {
    fn default() -> Self {
        Self {
            header: Default::default(),
            object_id: INodeNo(0),
            sequence_number: Default::default(),
            children: Default::default(),
            block_tree: Default::default(),
            block_tree_height: Default::default(),
        }
    }
}

unsafe impl Send for INode {}
unsafe impl Sync for INode {}
