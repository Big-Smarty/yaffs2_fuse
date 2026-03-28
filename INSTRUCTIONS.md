# HOW TO BUILD
```bash
git clone https://github.com/Big-Smarty/yaffs2_fuse.git
cd yaffs2_fuse
cargo build --release
```

# HOW TO RUN
```bash
Usage: yaffs2_fuse [OPTIONS] --image <IMAGE> --mount-point <MOUNT_POINT>

Options:
  -i, --image <IMAGE>              
  -m, --mount-point <MOUNT_POINT>  
  -p, --page-size <PAGE_SIZE>      [default: 2048]
  -o, --oob-size <OOB_SIZE>        [default: 64]
  -e, --erase-size <ERASE_SIZE>    [default: 131072]
  -t, --tags-offset <TAGS_OFFSET>  [default: 30]
  -h, --help                       Print help
  -V, --version                    Print version
```

### example

```bash
sudo ./target/release/yaffs2_fuse --image nexus.nanddump --mount-point /mnt
```

# HOW TO USE
The mount point can only be accessed as root. Thus, you should run:
```bash
sudo su
cd /mnt
<do whatever you want>
```

# Defaults
The YAFFS2 spec has very very loose definitions of the OOB tag on-drive structure.
It also does not contain any magic values or whatever to indicate the layout.
Thus, the driver either has to guess or use configurations.
This driver goes with the latter approach: a cli-based configuration.
The defaults are set for nexus.nanddump given to us in digital forensics class.