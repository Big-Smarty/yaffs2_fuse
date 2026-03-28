#[derive(Copy, Clone, Debug, Default)]
pub struct Tags {
    pub sequence_number: u64,
    pub object_id: u64,
    pub chunk_id: u64,
    pub num_data_bytes: u64,
    pub is_header: bool,
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug, Default)]
pub struct PackedTags {
    pub sequence_number: u32,
    pub object_id: u32,
    pub chunk_id: u32,
    pub num_data_bytes: u32,
}

impl Into<Tags> for PackedTags {
    fn into(self) -> Tags {
        Tags {
            sequence_number: self.sequence_number as u64,
            object_id: self.object_id as u64 & 0x0FFFFFFF,
            chunk_id: self.chunk_id as u64 & 0x0FFFFFFF,
            num_data_bytes: self.num_data_bytes as u64,
            is_header: (self.chunk_id & 0x80000000) != 0,
        }
    }
}
