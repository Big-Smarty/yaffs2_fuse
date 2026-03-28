#[derive(Copy, Clone, Debug)]
pub struct Config {
    pub page_size: usize,
    pub extra_size: usize,
    pub erase: usize,
    pub chunks_per_block: usize,
    pub nblocks: usize,
    pub nchunks: usize,
    pub block_size: usize,
    pub tags_offset: usize,
}

impl Config {
    pub fn new(
        device_size: usize,
        mtd_page_size: usize,
        mtd_extra_size: usize,
        mtd_erase: usize,
        offset: usize,
    ) -> Self {
        Self {
            page_size: mtd_page_size,
            extra_size: mtd_extra_size,
            erase: mtd_erase,
            chunks_per_block: mtd_erase / mtd_page_size,
            nblocks: device_size
                / ((mtd_erase / mtd_extra_size) * (mtd_page_size + mtd_extra_size)),
            nchunks: (device_size
                / ((mtd_erase / mtd_extra_size) * (mtd_page_size + mtd_extra_size)))
                * (mtd_erase / mtd_extra_size),
            block_size: mtd_page_size + mtd_extra_size,
            tags_offset: offset,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            page_size: 2048,
            extra_size: 64,
            erase: 131072,
            chunks_per_block: 131072 / 64,
            nblocks: 0,
            nchunks: 0,
            block_size: 2112,
            tags_offset: 30,
        }
    }
}
