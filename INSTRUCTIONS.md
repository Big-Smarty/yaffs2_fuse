# How to build
```bash
git clone https://github.com/Big-Smarty/yaffs2_fuse.git
cd yaffs2_fuse
cargo build --release
```

# How to run
```bash
sudo ./target/release/yaffs2_fuse --image <IMAGE> --mount-point <MOUNT_POINT>
```

# How to run prebuilt binary
```bash
sudo ./yaffs2_fuse --image <IMAGE> --mount-point <MOUNT_POINT>
```

# Example:
```bash
sudo ./yaffs2_fuse --image nexus.nandump --mount-point /mnt
```

