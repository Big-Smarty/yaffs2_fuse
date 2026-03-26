#[repr(C)]
#[repr(packed)]
#[derive(Copy, Clone, Debug, Default)]
pub struct Tags {
    pub sequence_number: u32,
    pub object_id: u32,
    pub chunk_id: u32,
    pub num_data_bytes: u32,
}
