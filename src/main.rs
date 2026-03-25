use std::io::ErrorKind;
use std::path::PathBuf;

use env_logger::Builder;
use fuser::SessionACL;
use fuser::{Config, MountOption};
use log::{error, warn};
use yaffs2_fuse::yaffs2::util::fuse_allow_other_enabled;
use yaffs2_fuse::yaffs2::*;

fn main() {
    Builder::new().filter_level(log::LevelFilter::Info).init();
    let mut cfg = Config::default();
    cfg.mount_options = vec![MountOption::FSName("YAFFS2".to_string())];
    if let Ok(enabled) = fuse_allow_other_enabled() {
        if enabled {
            cfg.acl = SessionACL::All;
        }
    } else {
        eprintln!("Unable to read /etc/fuse.conf");
    }

    warn!("working!");

    let result = fuser::mount2(
        match Yaffs2::new(PathBuf::from("nexus.nandump")) {
            Ok(yfs) => yfs,
            Err(e) => {
                panic!("{e}");
            }
        },
        "/mnt",
        &cfg,
    );

    match result {
        Ok(_) => (),
        Err(e) => match e.kind() {
            ErrorKind::PermissionDenied => {
                error!("{e}; run as root!");
            }
            _ => {
                error!("{e}");
            }
        },
    }
}
