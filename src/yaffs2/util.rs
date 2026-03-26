use std::io::BufRead;
use std::{
    fs::File,
    io::{self},
    os::unix::fs::FileExt,
};

pub fn bread(block_size: u64, block: u64, file: &File) -> io::Result<Vec<u8>> {
    let mut out = vec![0; block_size as usize];
    match file.read_at(&mut out, block * block_size) {
        Ok(bc) => {
            assert_eq!(bc, block_size as usize);
            Ok(out)
        }
        Err(e) => Err(e),
    }
}

pub fn fuse_allow_other_enabled() -> io::Result<bool> {
    let file = File::open("/etc/fuse.conf")?;
    for line in io::BufReader::new(file).lines() {
        if line?.trim_start().starts_with("user_allow_other") {
            return Ok(true);
        }
    }
    Ok(false)
}
