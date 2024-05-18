/*
    shell/fs.rs
*/

pub mod hd;
use hd::Hd;
use hd::HdErr;
use hd::Sector;

const BLK_SIZE      : usize = 0x1000;           // 4 kB blocks
const SECT_SIZE     : usize = hd::SECT_SIZE ;   // 512 bytes sectors
const INODE_SIZE    : usize = 0x100 ;           // 256 bytes inodes
const FDESC_SIZE    : usize = 0x40;             // 64 bytes file descriptor


const SECT_PER_BLK  : usize = BLK_SIZE/SECT_SIZE;  // = 8
const INODE_PER_BLK : usize = BLK_SIZE/INODE_SIZE; // = 16
const FDESC_PER_BLK : usize = BLK_SIZE/FDESC_SIZE; // = 64


const DIRECT_BLK    : usize = 14;
const MAX_NAME_LEN  : usize = 32;

const EMPTY_FILE : Fdesc = Fdesc { 
    name_len : 0,
    name     : ['\0'; MAX_NAME_LEN],
    iid : 0,
};
const EMPTY_SUPER: Super = Super {
    blk_nb   : 0,
    dblk_nb  : 0,
    iblk_nb  : 0,
    imap_sz  : 0,
    dmap_sz  : 0,
    inodes   : 0,
    datas    : 0,
    imap     : 0,
    dmap     : 0,

    root : Inode {
        id      : 0,
        ftype   : FType::Reg,
        size    : 0,
        data_ptr : [0;DIRECT_BLK + 1],
    },
};

// Some utils (most are unsafe, but expected to be used only in a secure context)
#[inline(always)]
fn select_bit(byte : u8, bit : usize) -> bool {
    ((byte<<bit)>>7) == 1
}
#[inline(always)]
fn change_bit(byte : u8, bit : usize, val: bool) -> u8 {
    if val  {byte | (0b1000_0000>>bit)}
    else    {byte & (0b1111_1111 ^ (0b1000_0000>>bit))}
}
#[inline(always)]
fn word_from_bytes(a : u8, b : u8, c : u8, d : u8) -> u32 {
    ((a as u32) <<24) + ((b as u32) <<16) + ((c as u32) <<8) + (d as u32)
}
#[inline(always)]
fn bytes_from_word(w : u32) -> (u8,u8,u8,u8) {
    ((w>>24) as u8, (w>>16) as u8, (w>>8) as u8,w as u8)
}
#[inline(always)]
fn fill(buff : &mut [u8], data : u32, ofs : usize){
    let (a,b,c,d) = bytes_from_word(data);
    buff[ofs + 0] = a;
    buff[ofs + 1] = b;
    buff[ofs + 2] = c;
    buff[ofs + 3] = d;
}
#[inline(always)]
fn fetch(buff : &[u8], ofs : usize) -> u32 {
    word_from_bytes(
        buff[ofs],
        buff[ofs+1],
        buff[ofs+2],
        buff[ofs+3])
}
#[inline(always)]
fn is_alpha_num(c : char) -> bool {
    (c >= 'A' && c <= 'Z') 
    || (c >= 'a' && c <= 'z') 
    || (c >= '0' && c <= '9')
}
#[inline(always)]
fn ceil(a : usize, b : usize) -> usize{
    if a%b == 0 {a/b}
    else {a/b + 1}
}

fn name_from(string : &str) -> Result<[char; MAX_NAME_LEN],FsErr>{
    if string.len() > MAX_NAME_LEN {return Err(FsErr::InvalidName)}
    let mut name = ['\0'; MAX_NAME_LEN];
    let mut buff = string.chars();
    let mut k = 0;
    while let Some(c) = buff.next() {
        if !is_alpha_num(c) 
        && c != '_' 
        && c != '\0' 
        && c != '.'
        {return Err(FsErr::InvalidName)}
        name[k] = c;
        k += 1;
    }
    Ok(name)
}
fn unsafe_name_from(string : &str) -> [char; MAX_NAME_LEN]{
    let mut name = ['\0'; MAX_NAME_LEN];
    let mut buff = string.chars();
    let mut k = 0;
    while let Some(c) = buff.next() {
        name[k] = c;
        k += 1;
    }
    name
}

type Block  = [u8;BLK_SIZE];
pub type Format = Vec<char>;

struct Bitmap {
    bmap : [u8;BLK_SIZE], // convention : 1 for not free, 0 for free
}

impl Bitmap {
    // -> UNSAFE : need to check the return value !
    fn find_free(&self) -> usize {
        let mut count : usize = 0;
        for byte in self.bmap {
            for bit in 0..8{
                if !select_bit(byte, bit){
                    let idx = count + bit;
                    if idx > 0  {return idx} 
                    else        {continue}  // to avoid fill inode 0
                }
            }
            count += 8;
        }
        return BLK_SIZE + 1;
    }
    
    #[inline] // -> UNSAFE : need to check the input value to avoid double free
    fn free(&mut self, idx : usize){
        let byte = self.bmap[idx/8];
        let byte = change_bit(byte,idx%8, false);
        self.bmap[idx/8] = byte;
    }

    #[inline] // -> UNSAFE : need to check the input value to avoid double unfree
    fn unfree(&mut self, idx : usize){
        let byte = self.bmap[idx/8];
        let byte = change_bit(byte,idx%8, true);
        self.bmap[idx/8] = byte;
    }

    #[inline]
    fn is_free(&self, idx : usize) -> bool {
        if idx == BLK_SIZE + 1 {return false}
        let byte = self.bmap[idx/8];
        return select_bit(byte,idx%8);
    }
}

#[derive(Debug)] // TODO : remove it
pub enum FsErr {
    HdErr(HdErr),
    InvalidName,
    FileNotFound,
    NoDirectory,
    Occuped,
    ReadDir,
    WriteDir,
    FileExist,
    RemoveDir,
    DirFull,
    ImapFull,
    DmapFull,
    UndefBlk,
    InvalidCur,
    MvCurOrPrev,
}

#[derive(Debug)] // TODO : remove it
enum FType {
    Reg,
    Dir,
    Undef,
}

#[derive(Debug)] // TODO : remove it
struct Inode {
    id      : u32,
    ftype   : FType,
    size    : usize, // size of corresponding file (in blocks)
    data_ptr : [u32;DIRECT_BLK + 1], // 1 indirection (in blocks)
}

impl Inode {
    fn hard_coded(&self) -> [u8; INODE_SIZE] {
        let mut hc : [u8; INODE_SIZE] = [0; INODE_SIZE];

        fill(&mut hc, self.id, 0);
        fill(&mut hc, self.size as u32,4);

        let ofs = 8;
        hc[ofs] = match self.ftype {
            FType::Reg => 0b0000_1000, // random for the moment (TODO)
            FType::Dir => 0b0000_1111,
            FType::Undef => 0b0000_0000,
        };

        for k in 0..(DIRECT_BLK + 1){
            let ofs = INODE_SIZE - 4*(DIRECT_BLK + 1 - k);
            fill(&mut hc, self.data_ptr[k], ofs);
        }
        return hc
    }

    fn from(hc : [u8; INODE_SIZE]) -> Self {
        Inode {
            id : fetch(&hc, 0),
            size : fetch(&hc, 4) as usize,
            ftype : match hc[8] {
               0b0000_1000 => FType::Reg,
               0b0000_1111 => FType::Dir,
               _ => FType::Undef,
            },
            data_ptr : {
                let mut data_ptr = [0;DIRECT_BLK + 1];
                for k in 0..DIRECT_BLK + 1 {
                    let ofs = INODE_SIZE - 4*(DIRECT_BLK+1 - k);
                    data_ptr[k] = fetch(&hc, ofs);
                }
                data_ptr
            },
        }
    }
}

#[derive(Debug)] // TODO : remove it
struct Super {
    blk_nb   : usize, // total blocks number
    dblk_nb  : usize, // data  blocks number
    iblk_nb  : usize, // inode blocks number
    imap_sz  : usize, // size of inode bitmap (in blocks)
    dmap_sz  : usize, // size of data  bitmap (in blocks)
    inodes   : u32,   // emplacement of first inode (in blocks)
    datas    : u32,   // emplacement of first data (in blocks)
    imap     : u32,   // emplacement of imap (in blocks)
    dmap     : u32,   // emplacement of dmap (in blocks)

    root : Inode,
}

impl Super {
    fn hard_coded(&self) -> Block {
        let mut hc : Block = [0;BLK_SIZE];

        let root_inode = self.root.hard_coded();
        for k in 0..INODE_SIZE {
            hc[k] = root_inode[k];
        }

        fill(&mut hc,self.blk_nb as u32,INODE_SIZE);
        fill(&mut hc,self.dblk_nb as u32,INODE_SIZE+4);
        fill(&mut hc,self.iblk_nb as u32,INODE_SIZE+8);
        fill(&mut hc,self.imap_sz as u32,INODE_SIZE+12);
        fill(&mut hc,self.dmap_sz as u32,INODE_SIZE+16);

        fill(&mut hc,self.inodes,INODE_SIZE+20);
        fill(&mut hc,self.datas,INODE_SIZE+24);
        fill(&mut hc,self.imap,INODE_SIZE+28);
        fill(&mut hc,self.dmap,INODE_SIZE+32);
        
        return hc
    }

    fn from(blk: Block) -> Self {
        let mut hc_inode = [0; INODE_SIZE];
        for k in 0..INODE_SIZE {
            hc_inode[k] = blk[k];
        }
        Super {
            blk_nb   : fetch(&blk, INODE_SIZE) as usize,
            dblk_nb  : fetch(&blk, INODE_SIZE+4) as usize,
            iblk_nb  : fetch(&blk, INODE_SIZE+8) as usize,
            imap_sz  : fetch(&blk, INODE_SIZE+12) as usize,
            dmap_sz  : fetch(&blk, INODE_SIZE+16) as usize,
            inodes   : fetch(&blk, INODE_SIZE+20),
            datas    : fetch(&blk, INODE_SIZE+24),
            imap     : fetch(&blk, INODE_SIZE+28),
            dmap     : fetch(&blk, INODE_SIZE+32),

            root : Inode::from(hc_inode),
        }
    }
}

#[derive(Debug)] // TODO : remove it
pub struct Fdesc {
    name_len : usize,
    name     : [char; MAX_NAME_LEN],
    iid : u32,
}

#[test] // TODO : remove it
impl std::fmt::Display for Fdesc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> core::fmt::Result {
        let name: std::string::String = self.name.iter().collect();
        write!(f, "
        Fdesc {{
            name_len : {}
            name : {}
            iid : {}
        }}", self.name_len, name, self.iid)
    }
}

impl Fdesc {
    fn hard_coded(&self) -> [u8; FDESC_SIZE]{
        let mut hc : [u8; FDESC_SIZE] = [0; FDESC_SIZE];

        let ofs = 0;
        let (a,b,c,d) = bytes_from_word(self.iid);
        hc[ofs + 0] = a;
        hc[ofs + 1] = b;
        hc[ofs + 2] = c;
        hc[ofs + 3] = d;

        let ofs = 4;
        for k in 0..MAX_NAME_LEN{
            hc[ofs + k] = self.name[k] as u8;
        }

        let ofs = 4 + MAX_NAME_LEN;
        hc[ofs] = self.name_len as u8;

        return hc
    }

    fn from(hc : [u8; FDESC_SIZE]) -> Self{
        Self {
            name_len : hc[4+MAX_NAME_LEN] as usize,
            name     : {let mut name = ['\0'; MAX_NAME_LEN];
                for k in 0..MAX_NAME_LEN{
                    name[k] = hc[4+k] as char;
                } name },
            iid : word_from_bytes(hc[0],hc[1],hc[2],hc[3]),
        }
    }

    fn copy(&self) -> Self{
        let mut copy = Self {
            name_len : self.name_len,
            name : ['\0'; MAX_NAME_LEN],
            iid : self.iid,
        };
        for k in 0..self.name_len {
            copy.name[k] = self.name[k];
        }
        copy
    }
}

#[derive(Debug)] // TODO : remove it
struct Dir {
    desc     : Fdesc,
    desc_tbl : [Fdesc; FDESC_PER_BLK],
    capacity : usize,
}

impl Dir {
    fn hard_coded(&self) -> Block {
        let mut hc : Block = [0;BLK_SIZE];
        for i in 0..FDESC_PER_BLK{
            let fdesc = self.desc_tbl[i].hard_coded();
            for j in 0..FDESC_SIZE{
                hc[i*FDESC_SIZE+j] = fdesc[j];
            }
        }
        return hc
    }

    fn from(blk : Block) -> Self {
        let mut desc_tbl = [EMPTY_FILE; FDESC_PER_BLK];
        for i in 0..FDESC_PER_BLK{
            let mut hc_fdesc = [0;FDESC_SIZE];
            for j in 0..FDESC_SIZE{
                hc_fdesc[j] = blk[i*FDESC_SIZE + j];
            }
            desc_tbl[i] = Fdesc::from(hc_fdesc);
        }

        let mut capacity = FDESC_PER_BLK;
        for k in 0..FDESC_PER_BLK{
            if desc_tbl[k].iid == 0 {
                capacity = k; 
                break;
            }
        }

        Dir {
            desc : EMPTY_FILE,
            desc_tbl : desc_tbl,
            capacity : capacity,
        }
    }

    fn new(new : Fdesc, parent : &Fdesc) -> Self{
        let mut dir = Dir {
            desc     : // other possibility : new.copy(),
            Fdesc {
                name_len : new.name_len,
                name : ['\0'; MAX_NAME_LEN],
                iid : new.iid,
            },
            desc_tbl : [EMPTY_FILE; FDESC_PER_BLK],
            capacity : 2,
        };
        // if not new.copy()
        for k in 0..new.name_len {
            dir.desc.name[k] = new.name[k];
        }
        dir.desc_tbl[0] = Fdesc {
            name_len : 1,
            name     : unsafe_name_from("."),
            iid : new.iid,
        };
        dir.desc_tbl[1] = Fdesc {
            name_len : 2,
            name     : unsafe_name_from(".."),
            iid : parent.iid,
        };
        return dir
    }

    fn find_free(&self) -> Result<usize, FsErr>{
        for k in 0..FDESC_PER_BLK{
            if self.desc_tbl[k].iid == 0 {return Ok(k)}
        }
        return Err(FsErr::DirFull)
    }

    fn find_file(&self, name : &str) -> Result<usize,FsErr>{
        let name = name_from(name)?;
        for k in 0..FDESC_SIZE{

            let file = &self.desc_tbl[k];
            let mut c = 0;
            while c<MAX_NAME_LEN {
                if file.name[c] != name[c] {break}
                if name[c] == '\0' {return Ok(k)}
                c += 1;
            }
            if c == (MAX_NAME_LEN-1) {return Ok(k)}
        }
        return Err(FsErr::FileNotFound)
    }
}

pub struct Fs <'a> {
    hd  : &'a mut Hd,   // the mounted hard drive
    sup : Super,        // the corresponding super bloc
    imap: Bitmap,
    dmap: Bitmap,
}

impl <'a> Fs <'a> {

    pub fn mkfs(hd : &'a mut Hd) -> Option<FsErr> {

        // construct the abstract file sytem
        const ROOT : Inode = Inode {
            id      : 1,
            ftype   : FType::Dir,
            size    : 1,
            data_ptr : [8,0,0,0,0,
            0,0,0,0,0,
            0,0,0,0,0],
        };
        const SUPER : Super  = Super {
            blk_nb   : 64,
            dblk_nb  : 56,
            iblk_nb  : 5,
            imap_sz  : 1,
            dmap_sz  : 1,
        
            inodes   : 3,
            datas    : 8,
            imap     : 1,
            dmap     : 2,
        
            root : ROOT,
        };
        let mut imap = Bitmap {bmap : [0;BLK_SIZE]};
        let mut dmap = Bitmap {bmap : [0;BLK_SIZE]};
        imap.unfree(1);
        dmap.unfree(0);

        let mut fs = Fs {
            hd : hd,
            sup : SUPER,
            imap : imap,
            dmap : dmap,
        };

        // write it on hard drive

        if let Some(err) = fs.write_tbls()   {return Some(err)};
        if let Some(err) = fs.write_super() {return Some(err)};

        // Construct & write the root directory
        if let Some(err) = fs.write_inode(&ROOT) {return Some(err)};

        let root_desc = Fdesc {
            name_len : 1,
            name : unsafe_name_from("/"),
            iid : 1,
        };
        let copy = &root_desc.copy();
        let root_dir = Dir::new(root_desc, copy);
        if let Some(err) = fs.write_dir(&root_dir) {return Some(err)};

        return None
    }

    pub fn mount(hd : &'a mut Hd) -> Result<Fs<'a>,FsErr> {
        let mut fs = Fs {
            hd : hd,
            sup : EMPTY_SUPER,
            imap : Bitmap{bmap : [0;BLK_SIZE]},
            dmap : Bitmap{bmap : [0;BLK_SIZE]},
        };
        if let Some(err) = fs.read_super() {return Err(err)};
        if let Some(err) = fs.read_tbls()  {return Err(err)};
        return Ok(fs)
    }


    fn write_blk(&mut self, blk: Block, offset : u32) -> Option<FsErr>{
        for i in 0..SECT_PER_BLK{
            let mut sect : Sector = [0;SECT_SIZE];
            for j in 0..SECT_SIZE{
                sect[j] = blk[i*SECT_SIZE + j];
            }
            match self.hd.dwrite(offset*(SECT_PER_BLK as u32)+(i as u32), sect) {
                Some(err) => return Some(FsErr::HdErr(err)),
                None => continue,
            };
        }
        return None
    }

    fn write_tbls(&mut self) -> Option<FsErr> {
        if let Some(err) = self.write_blk(self.imap.bmap,1) {return Some(err)}
        self.write_blk(self.dmap.bmap,2)
    }

    fn write_super(&mut self) -> Option<FsErr>{
        self.write_blk(self.sup.hard_coded(), 0)
    }

    fn write_inode(&mut self, inode : &Inode) -> Option<FsErr>{
        let iid = inode.id;
        let hc = inode.hard_coded();
        let ofs = self.sup.inodes + iid/(INODE_PER_BLK as u32);
        let mut blk = match self.read_blk(ofs){
            Ok(blk) => blk,
            Err(err) => return Some(err),
        };
        for k in 0..INODE_SIZE{
            blk[((iid as usize)%INODE_PER_BLK)*INODE_SIZE+k] = hc[k]
        }
        self.write_blk(blk, ofs)
    }

    fn write_fblk(&mut self, iid : u32, blk_nm : usize, blk : Block) -> Option<FsErr>{
        let inode = match self.read_inode(iid){
            Ok(inode) => inode,
            Err(err)  => return Some(err),
        };
        if blk_nm >= inode.size {return Some(FsErr::UndefBlk)}

        if blk_nm < DIRECT_BLK {
            if let Some(err) = self.write_blk(blk, inode.data_ptr[blk_nm]) {return Some(err)}
        }
        else {
            let indirection = match self.read_blk(inode.data_ptr[DIRECT_BLK]){
                Ok(ind) => ind,
                Err(err) => return Some(err),
            };
            let ofs = 4*(blk_nm - DIRECT_BLK); // each addr is 4 bytes
            let ofs = fetch(&indirection, ofs as usize);
            if let Some(err) = self.write_blk(blk, ofs) {return Some(err)}
        }
        return None
    }

    fn write_dir(&mut self, dir : &Dir) -> Option<FsErr>{
        if let Some(err) = self.write_fblk(dir.desc.iid, 0, dir.hard_coded()){
            return Some(err)
        }
        None
    }

    fn write_file(&mut self, cur : &mut Dir, name : &str, data : &Format) -> Option<FsErr>{
        // access to the wanted inode
        let iid = cur.desc_tbl[
            match cur.find_file(name){
                Ok(i)    => i,
                Err(err) => return Some(err)
            }].iid;
        let mut inode = match self.read_inode(iid) {
            Ok(inode) => inode,
            Err(err)  => return Some(err)
        };
        if let FType::Dir = inode.ftype {return Some(FsErr::WriteDir)};
        inode.ftype = FType::Reg;

        // compute and add the good number of blocks
        let old_size = inode.size;
        inode.size = ceil(data.len(), BLK_SIZE);
        for k in old_size..inode.size {
            let did = self.dmap.find_free();
            if did >= self.sup.dblk_nb {return Some(FsErr::DmapFull)};
            self.dmap.unfree(did as usize);
            if let Some(err) = self.write_tbls() {return Some(err)};

            if k < DIRECT_BLK {
                inode.data_ptr[k] = did as u32 + self.sup.datas
            } 
            else {
                let mut indirection = match self.read_blk(inode.data_ptr[DIRECT_BLK]){
                    Ok(ind) => ind,
                    Err(err) => return Some(err),
                };
                fill(&mut indirection, did as u32 + self.sup.datas, 4*(k-DIRECT_BLK));
            }
        }
        if let Some(err) = self.write_inode(&inode) {return Some(err)};

        // fill the file with new data
        for i in 0..inode.size{
            let mut blk : Block = [0;BLK_SIZE];
            for j in 0..BLK_SIZE {
                let k = i*BLK_SIZE+j;
                if k < data.len() {blk[j] = data[k] as u8}
            }
            if let Some(err) = self.write_fblk(inode.id, i, blk) {return Some(err)};
        }
        None
    }


    fn read_blk(&mut self, offset :u32) -> Result<Block, FsErr>{
        let mut blk : Block = [0;BLK_SIZE];
        for i in 0..SECT_PER_BLK{
            let sect = match self.hd.dread(offset*(SECT_PER_BLK as u32)+(i as u32)) {
                Ok(sect) => sect,
                Err(err) => return Err(FsErr::HdErr(err)),
            };
            for j in 0..SECT_SIZE{
                blk[i*SECT_SIZE + j] = sect[j];
            }
        }
        Ok(blk)
    }

    fn read_tbls(&mut self) -> Option<FsErr>{
        self.imap.bmap = match self.read_blk(self.sup.imap) {
            Ok(bmap) => bmap,
            Err(err) => return Some(err),
        };
        self.dmap.bmap = match self.read_blk(self.sup.dmap) {
            Ok(bmap) => bmap,
            Err(err) => return Some(err),
        };
        return None
    }
    
    fn read_super(&mut self) -> Option<FsErr>{
        self.sup = match self.read_blk(0){
            Ok(sup) => Super::from(sup),
            Err(err) => return Some(err),
        };
        return None
    }

    fn read_inode(&mut self, iid: u32) -> Result<Inode,FsErr>{
        let blk : Block = self.read_blk(self.sup.inodes + iid/(INODE_PER_BLK as u32))?;
        let mut hc = [0;INODE_SIZE];
        for k in 0..INODE_SIZE {
            hc[k] = blk[((iid as usize)%INODE_PER_BLK)*INODE_SIZE+k]
        }
        Ok(Inode::from(hc))
    }

    fn read_fblk(&mut self, iid : u32, blk_nm : usize) -> Result<Block,FsErr>{
        let inode = self.read_inode(iid)?;
        if blk_nm >= inode.size {return Err(FsErr::UndefBlk)}

        if blk_nm < DIRECT_BLK {
            return Ok(self.read_blk(inode.data_ptr[blk_nm])?);
        }
        else {
            let indirection = self.read_blk(inode.data_ptr[DIRECT_BLK])?;
            let ofs = 4*(blk_nm - DIRECT_BLK); // 4 bytes addr
            return Ok(self.read_blk(fetch(&indirection, ofs as usize))?)
        }
    }

    fn read_dir(&mut self, iid : u32) -> Result<Dir,FsErr>{
        let dir_inode = self.read_inode(iid)?;
        let mut dir = match dir_inode.ftype {
            FType::Dir => Dir::from(self.read_fblk(dir_inode.id,0)?),
            _ => return Err(FsErr::NoDirectory),
        };
        dir.desc = Fdesc {
            name : unsafe_name_from("."),
            name_len : 1,
            iid : iid,
        };
        return Ok(dir);
        
    }


    fn mkdir__(&mut self, cur : &mut Dir, name : &str) -> Option<FsErr>{
        // check validity of the current directory
        let iid = cur.desc.iid;
        if !self.imap.is_free(iid as usize) {return Some(FsErr::InvalidCur)};

        // reject if current directory is full or the name is already used
        if cur.capacity >= FDESC_PER_BLK {return Some(FsErr::DirFull)}
        if let Ok(_) = cur.find_file(name) {return Some(FsErr::FileExist)}
    
        // find free data & inode blocks 
        let iid = self.imap.find_free();
        if iid >= self.sup.iblk_nb*INODE_PER_BLK {
            return Some(FsErr::ImapFull)
        };
        let did = self.dmap.find_free();
        if did >= self.sup.dblk_nb {
            return Some(FsErr::DmapFull)
        };
        // mark them unfree in imap & dmap and write tables
        self.imap.unfree(iid as usize);
        self.dmap.unfree(did as usize);
        if let Some(err) = self.write_tbls() {return Some(err)};
    
        // create and write a new inode
        let mut data_ptr = [0;DIRECT_BLK+1];
        data_ptr[0] = (did as u32) + self.sup.datas;
        let new_inode = Inode {
            id    : iid as u32,
            ftype : FType::Dir,
            size  : 1,
            data_ptr : data_ptr,
        };
        if let Some(err) = self.write_inode(&new_inode) {return Some(err)}
    
        // complete the directory with . and .. & write data
        let new_desc = Fdesc{
            name_len : name.len(),
            name     : match name_from(name){
                Ok(name) => name,
                Err(err) => return Some(err),
            },
            iid : new_inode.id as u32,
        };
        let new_dir = Dir::new(new_desc, &cur.desc);
        if let Some(err) = self.write_dir(&new_dir) {return Some(err)}
    
        // update the current directory and write changes
        let free_desc = match cur.find_free(){
            Ok(fd) => fd,
            Err(err) => return Some(err),
        };
        cur.desc_tbl[free_desc] = new_dir.desc;
        cur.capacity += 1;
        if let Some(err) = self.write_dir(cur) {return Some(err)}
    
        return None
    }

    fn rmdir__(&mut self, cur : &mut Dir, name : &str) -> Option<FsErr>{

        fn clean_dir(fs : &mut Fs, dir : &mut Dir) -> Option<FsErr> {
            for k in 2..FDESC_PER_BLK{
                let iid = dir.desc_tbl[k].iid;
                if iid == 0 {continue};
                dir.desc_tbl[k] = EMPTY_FILE;
                match fs.read_dir(iid) {
                    // erase directory
                    Ok(mut sub_dir) => {
                        clean_dir(fs, &mut sub_dir);
                        let dinode = match fs.read_inode(iid){
                            Ok(di) => di,
                            Err(err) => return Some(err),
                        };
                        // Update bitmaps tables
                        fs.imap.free(dinode.id as usize);
                        fs.dmap.free(dinode.data_ptr[0] as usize);
                        if let Some(err) = fs.write_tbls() {return Some(err)};
                    },
                    // erase file
                    Err(FsErr::NoDirectory) => {
                        let finode = match fs.read_inode(iid){
                            Ok(inode) => inode,
                            Err(err) => return Some(err),
                        };

                        // Update bitmaps tables
                        fs.imap.free(finode.id as usize);
                        for k in 0..finode.size {
                            if k < DIRECT_BLK {fs.dmap.free(finode.data_ptr[k] as usize)}
                            else {
                                let indirection = match fs.read_blk(finode.data_ptr[DIRECT_BLK]){
                                    Ok(ind) => ind,
                                    Err(err) => return Some(err),
                                };
                                fs.dmap.free(fetch(&indirection, 4*(k-DIRECT_BLK)) as usize);
                            }
                        }
                        if let Some(err) = fs.write_tbls() {return Some(err)};
                    },
                    Err(err) => return Some(err)
                }
            }
            dir.capacity = 0;
            if let Some(err) = fs.write_dir(dir) {return Some(err)}
            return None
        }

        // Find the removed directory
        let idx = match cur.find_file(name){
            Ok(idx) => idx,
            Err(err) => return Some(err),
        };
        let inode = match self.read_inode(cur.desc_tbl[idx].iid){
            Ok(inode) => inode,
            Err(err) => return Some(err),
        };
        // Update the current directory
        cur.desc_tbl[idx] = EMPTY_FILE;
        cur.capacity -= 1;
        if let Some(err) = self.write_dir(cur) {return Some(err)}
        // Clean the removed directory
        let mut rm_dir = match self.read_dir(inode.id){
            Ok(dir) => dir,
            Err(err) => return Some(err)
        };
        clean_dir(self,&mut rm_dir);
        // Update bitmaps tables
        self.imap.free(inode.id as usize);
        self.dmap.free(inode.data_ptr[0] as usize);
        if let Some(err) = self.write_tbls() {return Some(err)};
        return None
    }

    fn touch__(&mut self, cur_dir : &mut Dir, name : &str)-> Option<FsErr> {
        // check validity of the current directory
        let iid = cur_dir.desc.iid;
        if !self.imap.is_free(iid as usize) {return Some(FsErr::InvalidCur)};

        // reject if current directory is full
        if cur_dir.capacity >= FDESC_PER_BLK {return Some(FsErr::DirFull)}
        if let Ok(_) = cur_dir.find_file(name) {return Some(FsErr::FileExist)}
    
        // find free data & inode blocks
        let iid = self.imap.find_free();
        if iid >= self.sup.iblk_nb*INODE_PER_BLK {
            return Some(FsErr::ImapFull)
        };
        let did = self.dmap.find_free();
        if did >= self.sup.dblk_nb {
            return Some(FsErr::DmapFull)
        };
    
        // mark them unfree in imap & dmap and write tables
        self.imap.unfree(iid as usize);
        self.dmap.unfree(did as usize);
        if let Some(err) = self.write_tbls() {return Some(err)};
    
        // create and write a new inode
        let mut data_ptr = [0;DIRECT_BLK+1];
        data_ptr[0] = (did as u32) + self.sup.datas;
        let file_inode = Inode {
            id    : iid as u32,
            ftype : FType::Reg,
            size  : 1,
            data_ptr : data_ptr,
        };
        if let Some(err) = self.write_inode(&file_inode) {return Some(err)}
    
        // update the current directory and write changes
        let file_desc = Fdesc {
            name_len : name.len(),
            name     : match name_from(name){
                Ok(name) => name,
                Err(err) => return Some(err),
            },
            iid : file_inode.id as u32,
        };
        let free_desc = match cur_dir.find_free(){
            Ok(fd) => fd,
            Err(err) => return Some(err),
        };
        cur_dir.desc_tbl[free_desc] = file_desc;
        cur_dir.capacity += 1;
        if let Some(err) = self.write_dir(cur_dir) {return Some(err)}
    
        return None
    }

    fn rm__(&mut self, cur : &mut Dir, name : &str) -> Option<FsErr>{

        // find the inode of the removed file
        let file_desc_idx = match cur.find_file(name){
            Ok(k) => k,
            Err(err) => return Some(err),
        };
        let file_inode = match self.read_inode(cur.desc_tbl[file_desc_idx].iid){
            Ok(oi) => oi,
            Err(err) => return Some(err),
        };

        if let FType::Dir = file_inode.ftype {return Some(FsErr::RemoveDir)};

        // Update bitmaps tables
        self.imap.free(file_inode.id as usize);
        for k in 0..file_inode.size {
            if k < DIRECT_BLK {self.dmap.free(file_inode.data_ptr[k] as usize)}
            else {
                let indirection = match self.read_blk(file_inode.data_ptr[DIRECT_BLK]){
                    Ok(ind) => ind,
                    Err(err) => return Some(err),
                };
                self.dmap.free(fetch(&indirection, 4*(k-DIRECT_BLK)) as usize);
            }
        }
        if let Some(err) = self.write_tbls() {return Some(err)};

        // Update the current directory
        cur.desc_tbl[file_desc_idx] = EMPTY_FILE;
        cur.capacity -= 1;
        if let Some(err) = self.write_dir(cur) {return Some(err)}
    
        return None
    }

    fn ls_dir(&mut self, dir : &Dir) -> Result<Format,FsErr>{
        let mut fmt : Format = Vec::new();
        for i in 0..FDESC_PER_BLK{

            let iid = dir.desc_tbl[i].iid;
            if iid == 0 {continue}
            let inode = match self.read_inode(iid){
                Ok(inode) => inode,
                Err(err) => return Err(err),
            };

            match inode.ftype {
                FType::Dir => { for j in 0..dir.desc_tbl[i].name_len {
                        fmt.push(dir.desc_tbl[i].name[j]);
                        //print!("\x1b[93m{}\x1b[0m",dir.desc_tbl[i].name[j]);
                    }
                },
                FType::Reg => { for j in 0..dir.desc_tbl[i].name_len {
                        fmt.push(dir.desc_tbl[i].name[j]);
                        //print!("{}",dir.desc_tbl[i].name[j]);
                    }
                },
                FType::Undef => { for j in 0..dir.desc_tbl[i].name_len {
                        fmt.push(dir.desc_tbl[i].name[j]);
                        //print!("\x1b[93m{}\x1b[0m",dir.desc_tbl[i].name[j]);
                    }
                }
            };
            fmt.push('\n');
        }
        return Ok(fmt)
    }

    fn cat_file(&mut self, cur_dir : &Dir, name : &str) -> Result<Format,FsErr>{
        let mut v : Format = Vec::new();
        let iid = cur_dir.desc_tbl[cur_dir.find_file(name)?].iid;
        let inode = self.read_inode(iid)?;
        if let FType::Dir = inode.ftype {return Err(FsErr::ReadDir)};
        for k in 0..inode.size{
            let blk = self.read_fblk(iid,k)?;
            for l in 0..BLK_SIZE{
                let c = blk[l] as char;
                if c == '\0' {v.push('\n'); return Ok(v)}
                v.push(c);
            }
        }
        Ok(v)
    }
}


#[derive(Debug)] // TODO : remove it
struct Path<'a> {
    cur : &'a str,
    next : &'a str,
    abs : bool,
}

impl <'a> Path<'a> {
    fn from(path : &'a str) -> Path<'a> {
        let mut idx = 0;
        for c in path.chars() {

            if (c == '/') && (idx == 0) {
                return Path {
                    cur : &path[..1],
                    next : &path[1..],
                    abs : true,
                }
            }

            else if c == '/' {
                return Path {
                    cur : &path[..idx],
                    next : &path[(idx+1)..],
                    abs : false,
                }
            }
            idx += 1;
        }
        return Path {
            cur : &path,
            next : &"",
            abs : false,
        }
    }
}

impl<'a> Fs<'a> {

    pub fn get_home_fdesc(&mut self) -> Fdesc {
        return Fdesc {
            name_len : 1,
            name     : unsafe_name_from("/"),
            iid : self.sup.root.id
        };
    }

    pub fn cd(&mut self, cur : &Fdesc, path: &str) -> Result<Fdesc,FsErr>{
        let path = Path::from(path);
        if path.abs {
            let root = self.read_dir(self.sup.root.id)?.desc;
            return self.cd(&root,path.next)
        }

        let cur = self.read_dir(cur.iid)?;
        if path.next == ""  {
            let fd = cur.desc_tbl[cur.find_file(path.cur)?].copy();
            match self.read_inode(fd.iid)?.ftype {
                FType::Dir => return Ok(fd),
                _ => return Err(FsErr::NoDirectory)
            }
        }
        
        let next = &cur.desc_tbl[cur.find_file(path.cur)?];
        return self.cd(next,path.next)
    }

    pub fn mv(&mut self, cur : &Fdesc, old_path : &str, new_path : &str) -> Option<FsErr>{
        fn _mv_(fs: &mut Fs, old_dir : &mut Dir, new_dir : &mut Dir, old_name : &str, new_name : &str) -> Option<FsErr>{
            // take and modify the file descriptor
            let old_idx = match old_dir.find_file(old_name) {
                Ok(idx) => idx,
                Err(err) => return Some(err)
            };
            let mut fd = old_dir.desc_tbl[old_idx].copy();
            fd.name = match name_from(new_name){
                Ok(name) => name,
                Err(err) => return Some(err)
            };
            fd.name_len = new_name.len();

            // case old_dir == new_dir : cannot have 2 fresh desc !
            if old_dir.desc.iid == new_dir.desc.iid {
                new_dir.desc_tbl[old_idx] = fd;
                if let Some(err) = fs.write_dir(new_dir) {return Some(err)};
                return None
            }

            // find a new emplacement inside the new directory
            let new_idx = match new_dir.find_free() {
                Ok(idx) => idx,
                Err(err) => return Some(err)
            };
            new_dir.capacity += 1;
            new_dir.desc_tbl[new_idx] = fd;
            if let Some(err) = fs.write_dir(new_dir) {return Some(err)}

            // remove the file from the old directory
            old_dir.desc_tbl[old_idx] = EMPTY_FILE;
            old_dir.capacity -= 1;
            if let Some(err) = fs.write_dir(old_dir) {return Some(err)}
            return None
        }

        // Find old directory and old name
        fn _chassing1_<'a,'b>(fs : &'b mut Fs, cur: &'b Fdesc, path : &'a str) -> Result<(Dir,&'a str),FsErr>{
            let path = Path::from(path);
            if path.abs {
                let root = fs.read_dir(fs.sup.root.id)?.desc;
                return _chassing1_(fs,&root, path.next)
            }

            let cur = fs.read_dir(cur.iid)?;
            if path.next == "" {return Ok((cur, path.cur))}
            let next = &cur.desc_tbl[cur.find_file(path.cur)?];
            return _chassing1_(fs,next,path.next)
        }

        // Find new directory and optionnal new name
        fn _chassing2_<'a,'b>(fs : &'b mut Fs, cur: &'b Fdesc, path : &'a str) -> Result<(Dir,Option<&'a str>),FsErr>{
            let path = Path::from(path);
            if path.abs {
                let root = fs.read_dir(fs.sup.root.id)?.desc;
                return _chassing2_(fs,&root, path.next)
            }

            let cur = fs.read_dir(cur.iid)?;
            if path.next == "" {
                match cur.find_file(path.cur) {
                    Ok(idx) => {let dir = fs.read_dir(cur.desc_tbl[idx].iid)?; return Ok((dir, None))},
                    Err(FsErr::FileNotFound) => return Ok((cur, Some(path.cur))),
                    Err(err) => return Err(err)
                }
            }
            let next = &cur.desc_tbl[cur.find_file(path.cur)?];
            println!("tada!");
            return _chassing2_(fs,next,path.next)
        }

        let chasse1 = match _chassing1_(self, cur, old_path){
            Ok(res) => res,
            Err(err) => return Some(err)
        };
        let chasse2 = match _chassing2_(self, cur, new_path){
            Ok(res) => res,
            Err(err) => return Some(err)
        };
        let (mut old_dir, mut new_dir, old_name, new_name) = match (chasse1, chasse2) {
            ((od,on),(nd,None))    => (od,nd,on,on),
            ((od,on),(nd,Some(nn))) => (od,nd,on,nn)
        };

        let mut buff = old_name.chars();
        if let Some('.') = buff.next() {
            let tmp = buff.next();
            if let None = tmp {return Some(FsErr::MvCurOrPrev)};
            if let Some('.') = tmp {
                if let None = buff.next() {return Some(FsErr::MvCurOrPrev)}
            }
        }
        return _mv_(self, &mut old_dir, &mut new_dir, old_name, new_name)
    }

    pub fn mkdir(&mut self, cur: &Fdesc, path : &str) -> Option<FsErr> {

        let path = Path::from(path);
        if path.abs {
            let root = match self.read_dir(self.sup.root.id) {
                Ok(dir) => dir.desc,
                Err(err) => return Some(err),
            };
            return Some(self.mkdir(&root, path.next)?)
        }

        let mut cur = match self.read_dir(cur.iid) {
            Ok(dir) => dir,
            Err(err) => return Some(err),
        };
        if path.next == "" {
            return Some(self.mkdir__(&mut cur, path.cur)?)
        }
        let next = &cur.desc_tbl[
            match cur.find_file(path.cur){
                Ok(i) => i,
                Err(err) => return Some(err),
            }
        ];
        return Some(self.mkdir(next,path.next)?)
    }

    pub fn rmdir(&mut self, cur: &Fdesc, path : &str) -> Option<FsErr> {

        let path = Path::from(path);
        if path.abs {
            let root = match self.read_dir(self.sup.root.id) {
                Ok(dir) => dir.desc,
                Err(err) => return Some(err),
            };
            return Some(self.rmdir(&root, path.next)?)
        }

        match path.cur {
            "." | ".." => return Some(FsErr::Occuped),

            _ => { let mut cur = match self.read_dir(cur.iid) {
                    Ok(dir) => dir,
                    Err(err) => return Some(err),
                };
                if path.next == "" {
                    return Some(self.rmdir__(&mut cur, path.cur)?)
                }
                let next = &cur.desc_tbl[
                    match cur.find_file(path.cur){
                        Ok(i) => i,
                        Err(err) => return Some(err),
                    }
                ];
                return Some(self.rmdir(next,path.next)?)
            }
        }      
    }
    
    pub fn touch(&mut self, cur: &Fdesc, path : &str) -> Option<FsErr> {

        let path = Path::from(path);
        if path.abs {
            let root = match self.read_dir(self.sup.root.id) {
                Ok(dir) => dir.desc,
                Err(err) => return Some(err),
            };
            return Some(self.touch(&root, path.next)?)
        }

        let mut cur = match self.read_dir(cur.iid) {
            Ok(dir) => dir,
            Err(err) => return Some(err),
        };
        if path.next == "" {
            return Some(self.touch__(&mut cur, path.cur)?)
        }
        let next = &cur.desc_tbl[
            match cur.find_file(path.cur){
                Ok(i) => i,
                Err(err) => return Some(err),
            }
        ];
        return Some(self.touch(next,path.next)?)
    }

    pub fn rm(&mut self, cur: &Fdesc, path : &str) -> Option<FsErr> {

        let path = Path::from(path);
        if path.abs {
            let root = match self.read_dir(self.sup.root.id) {
                Ok(dir) => dir.desc,
                Err(err) => return Some(err),
            };
            return Some(self.rm(&root, path.next)?)
        }


        let mut cur = match self.read_dir(cur.iid) {
            Ok(dir) => dir,
            Err(err) => return Some(err),
        };
        if path.next == "" {
            return Some(self.rm__(&mut cur, path.cur)?)
        }
        let next = &cur.desc_tbl[
            match cur.find_file(path.cur){
                Ok(i) => i,
                Err(err) => return Some(err),
            }
        ];
        return Some(self.rm(next,path.next)?)
    }

    pub fn ls(&mut self, cur: &Fdesc, path : &str) -> Result<Format,FsErr> {

        let path = Path::from(path);
        if path.abs {
            let root = self.read_dir(self.sup.root.id)?.desc;
            return self.ls(&root, path.next)
        }
        
        let cur = self.read_dir(cur.iid)?;
        if path.cur == "" {
            return self.ls_dir(&cur)
        }
        let next = &cur.desc_tbl[cur.find_file(path.cur)?];
        return self.ls(next,path.next)
        
    }

    pub fn write(&mut self, cur: &Fdesc, path : &str, data : &Format) -> Option<FsErr> {
        let path = Path::from(path);
        if path.abs {
            let root = match self.read_dir(self.sup.root.id) {
                Ok(dir) => dir.desc,
                Err(err) => return Some(err),
            };
            return Some(self.write(&root, path.next, data)?)
        }


        let mut cur = match self.read_dir(cur.iid) {
            Ok(dir) => dir,
            Err(err) => return Some(err),
        };
        if path.next == "" {
            return Some(self.write_file(&mut cur, path.cur, data)?)
        }
        let next = &cur.desc_tbl[
            match cur.find_file(path.cur){
                Ok(i) => i,
                Err(err) => return Some(err),
            }
        ];
        return Some(self.write(next,path.next,data)?)
    }

    pub fn cat(&mut self, cur: &Fdesc, path : &str) -> Result<Format,FsErr> {
        let path = Path::from(path);
        if path.abs {
            let root = self.read_dir(self.sup.root.id)?.desc;
            return self.cat(&root,path.next)
        }

        let cur = self.read_dir(cur.iid)?;
        if path.next == ""  {
            return self.cat_file(&cur, path.cur)
        }
        
        let next = &cur.desc_tbl[cur.find_file(path.cur)?];
        return self.cat(next,path.next)
    }

    pub fn grep(&mut self, cur: &Fdesc, path: &str, pattern: &str) -> Result<Format,FsErr> {
        // get the content & add a space a the end
        let mut content : Format = self.cat(cur, path)?;
        content.push(' ');
        // convert pattern to Format
        let mut fmt = Vec::new();
        let mut buff = pattern.chars();
        while let Some(c) = buff.next() {
            fmt.push(c);
        }
        let pattern = fmt;

        // begining of the research
        let mut start = 0; let mut end = 0;
        let mut res = vec![];
        while end < content.len() {
            if (content[end] == ' ') || (content[end] == '\n') {start = end+1;}

            else if content[end] == pattern[0] {
                if end + pattern.len() > content.len() {break}
                let mut k = 1;
                while k < pattern.len() {
                    if content[end+k] != pattern[k] {break}
                    k += 1;
                }
                if k == pattern.len() {
                    end += k;
                    while end < content.len() {
                        if (content[end] == ' ') || (content[end] == '\n') {break}
                        end += 1;
                    }
                    for l in start..end{
                        res.push(content[l])
                    }
                    res.push('\n');
                    start = end+1;
                }
            }

            end += 1;
        }
        return Ok(res)
    }
}
