#[derive(Clone, Debug)]
pub enum Tree {
    Internal(Box<[Option<Tree>; 8]>),
    Leaf(Box<[u32; 16]>),
}

impl Default for Tree {
    fn default() -> Self {
        Self::Internal(Box::new([const { None }; 8]))
    }
}
