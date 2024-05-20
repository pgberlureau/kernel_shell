# A Very Basic File System :

## Build it, run it :

`cargo build`

`cargo run`

## Arch :

### Disk

Disk size : 64 blocks (1 block = 4 kB)

`| super | imap | dmap | INODES (x5) | DATAS (x56) |`

### File system

- Super : `| blk_nb | dblk_nb | iblk_nb | imap_sz | dmap_sz | fst_inode | fst_data | imap | dmap | root (inode) | ... |`

- Inode : `| iid | ftype | fsize | data (*u8) | ... |`

- Directory : `| desc (fdesc) | desc_tbl (*fdesc) | capa |`

- File descriptor : `| name (*char) | iid | ... |`

### Shell

- Command : `cmd := cmd1 (pp_cmd) | ... | cmdn (pp_cmd)`

- Piped Command : `pp_cmd := cmd (sp_cmd) || cmd (sp_cmd) > file (file name)`

- Simple Command : `sp_cmd := {mkdir, touch, rmdir, rm, mv, cd, echo, cat, ls, grep, exit} {args}`


