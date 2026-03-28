mod args;

use args::*;
use clap::Parser;
use env_logger::Builder;
use fuser::{Config, MountOption};
use log::error;

fn main() {
    Builder::new().filter_level(log::LevelFilter::Trace).init();

    let args = Args::parse();
    let mut config = Config::default();
    config.mount_options = vec![
        MountOption::FSName("Yaffs2".to_string()),
        MountOption::RO,
        MountOption::Sync,
    ];
    config.acl = fuser::SessionACL::All;

    let result = fuser::mount2(
        match args.get_fs() {
            Ok(fs) => fs,
            Err(_) => panic!("failed to init fs driver!"),
        },
        args.mount_point,
        &config,
    );
    match result {
        Ok(_) => (),
        Err(e) => match e.kind() {
            std::io::ErrorKind::PermissionDenied => {
                error!("{e}\nrun as root!")
            }
            _ => {
                error!("{e}")
            }
        },
    };
}
