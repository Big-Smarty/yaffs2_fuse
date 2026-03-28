use std::{
    fs::File,
    io::{self},
};

use clap::Parser;
use memmap2::Mmap;
use yaffs2_fuse::yaffs2::Yaffs2;

#[derive(Parser, Clone, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    pub image: String,

    #[arg(short, long)]
    pub mount_point: String,

    #[arg(short, long, default_value_t = 2048)]
    pub page_size: usize,

    #[arg(short, long, default_value_t = 64)]
    pub oob_size: usize,

    #[arg(short, long, default_value_t = 131072)]
    pub erase_size: usize,

    #[arg(short, long, default_value_t = 30)]
    pub tags_offset: usize,
}

impl Args {
    pub fn get_fs(&self) -> io::Result<Yaffs2> {
        let file = File::open(self.image.clone())?;
        let mmap = unsafe { Mmap::map(&file)? };
        Yaffs2::new(
            mmap,
            self.page_size,
            self.oob_size,
            self.erase_size,
            self.tags_offset,
        )
    }
}
