# Dokumentation

Das ist meine Dokumentation für Prüfungsoption B Digitale Forensik.
Ziel ist es, einen eigenen Yaffs2 Treiber zu schreiben, um nexus.nanddump und 1.bin, eine Partition in a2019-gh2-full.bin auszulesen.
Hierfür wurde mithilfe von Rust ein Fuse-Treiber geschrieben, welcher mit verschiedenen Layouts umgehen kann.

# Demonstration

## nexus.nanddump
Ansicht in Helix:

![nexus_helix](images/yaffs2_helix.png)

LS von nexus.nanddump:

![nexus_ls](images/yaffs2_terminal_ls.png)

---

## 1.bin
Ansicht im Terminal:

![1.bin_ls](images/yaffs2_terminal_ls_1.bin.png)

Ansicht in Helix:

![1.bin_helix](images/yaffs2_helix_1.bin.png)

# Repo
https://github.com/Big-Smarty/yaffs2_fuse