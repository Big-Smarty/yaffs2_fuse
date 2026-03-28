// TODO: create a filedata struct containing a chunk vector (chunks being buffer references)
// TODO: add a filedata dashmap to Yaffs2 based on the object id

use dashmap::DashMap;
use fuser::INodeNo;
use log::{error, info};

use crate::yaffs2::object_header::Header;

pub struct FileData {
    pub object_id: INodeNo,
    pub header: Option<Header>,
    pub chunks: DashMap<usize, Vec<u8>>,
    pub size: usize,
}
impl FileData {
    pub fn new(object_id: INodeNo) -> Self {
        Self {
            object_id,
            header: None,
            chunks: DashMap::new(),
            size: 0,
        }
    }

    pub fn add_chunk(&mut self, chunk_id: usize, data: &Vec<u8>) {
        self.chunks.insert(chunk_id, data.clone());
    }

    pub fn is_complete(&self, page_size: usize) -> bool {
        if let Some(header) = &self.header {
            let expected_chunks = (header.size as usize + page_size - 1) / page_size;
            let has_all_chunks = self.chunks.len() == expected_chunks;
            info!(
                "File {}: size={}, page_size={}, expected_chunks={}, actual_chunks={}, complete={}",
                self.object_id,
                header.size,
                page_size,
                expected_chunks,
                self.chunks.len(),
                has_all_chunks
            );
            has_all_chunks
        } else {
            false
        }
    }

    pub fn reconstruct(&self, page_size: usize) -> Option<Vec<u8>> {
        if !self.is_complete(page_size) {
            return None;
        }
        if let Some(header) = &self.header {
            let mut file_data = Vec::with_capacity(header.size as usize);
            let chunks_per_file = (header.size as usize + page_size - 1) / page_size;
            for chunk_id in 1..=chunks_per_file {
                if let Some(chunk_data) = self.chunks.get(&chunk_id) {
                    let remaining = header.size as usize - file_data.len();
                    let take_bytes = remaining.min(chunk_data.len());
                    file_data.extend_from_slice(&chunk_data[..take_bytes]);
                } else {
                    error!("Missing chunk {} for file {}", chunk_id, self.object_id);
                    return None;
                }
            }
            Some(file_data)
        } else {
            None
        }
    }
}
