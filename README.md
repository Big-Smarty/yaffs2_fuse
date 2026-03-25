Fuckass yaffs2 fuse driver I made for a digital forensics class. Its basically https://github.com/bcopeland/fszoo translated to rust.

# HOWEVER.
the nandump we got was apparently nonstandard yaffs2, so while sleuthskit tools had no issue with it, this simple driver from 17 years ago did, in fact, have issues with it.
And as I am not known for my superb file system expertise and development skills, I had AI look over it and tell me what my mistakes were (not with translation but rather regarding that specific dump).

I am deeply ashamed by this admission and am thus going to rewrite ts in the next three days.
I also NEED to rewrite it because while it lists directories and files just fine, there are major issues with how files are actually read. so yeah.
I also hardcoded the path to the nandump and to the mount point for now. so yeah.
