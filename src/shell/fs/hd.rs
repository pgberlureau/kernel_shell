/*
    fs/hd.rs
*/

pub const SECT_SIZE : usize = 0x200 ;       // 512 bytes sectors
pub const HD_SIZE : usize = 64*8*SECT_SIZE; // 512 sectors hard drive (aka 64 blocks disk)
pub type Sector = [u8;SECT_SIZE];

enum HdState {
    Writing,
    Reading,
    Free,
}

#[derive(Debug)] // TODO : remove it
pub enum HdErr {
    Occuped,
}

pub struct Hd {
    state : HdState,
    array : [u8; HD_SIZE],
}

impl Hd {
    pub fn new() -> Hd {
        Hd {
            state : HdState::Free,
            array : [0;64*8*SECT_SIZE]
        }
    }

    #[test] // TODO : remove it
    pub fn init() -> Hd {
        let mut hd = Hd {
            state : HdState::Free,
            array : [0; 64*8*SECT_SIZE],
        };
        match std::fs::read("disk") {
            Ok(vec) => for k in 0..(64*8*SECT_SIZE) {
                hd.array[k] = vec[k];
            },
            Err(_) => panic!("Error during init disk"),
        }
        return hd
    }

    #[test] // TODO : remove it
    pub fn save(self) {
        std::fs::write("disk",self.array).expect("Error during saving disk");
    }

    #[test]
    pub fn display(&mut self, offset : u32) {
        let sect = match self.dread(offset){
            Ok(sect) => sect,
            Err(_) => panic!("Error when dislpaying disk !"),
        };
        for k in 0..32 {
            print!("0x{:04x} : ",k*16);
            for i in 0..8{
                for j in 0..2{
                    print!("{:02x}",sect[k*16+i*2+j]);
                }
                print!(" ");
            }
            for l in 0..16{
                let c = sect[k*16+l];
                print!("{}", if c > 32 {c as char} else {'.'});
            }
            println!("");
        }
        println!("");
    }

    pub fn dread(&mut self, sect_nb : u32) -> Result<Sector,HdErr> {

        let sect_nb = sect_nb as usize;
        let mut sect : Sector = [0;512];
        match self.state {
            HdState::Free => {
                self.state = HdState::Reading;
                for k in 0..SECT_SIZE {
                    sect[k] = self.array[k+SECT_SIZE*sect_nb];
                }; 
                self.state = HdState::Free;
                Ok(sect)
            }
            HdState::Reading => Err(HdErr::Occuped),
            HdState::Writing => Err(HdErr::Occuped),
        }
    }

    pub fn dwrite(&mut self, offset : u32, sect : Sector) -> Option<HdErr>{
        let offset = offset as usize;
        match self.state {
            HdState::Free => {
                self.state = HdState::Writing;
                for k in 0..SECT_SIZE{
                    self.array[k+offset*SECT_SIZE] = sect[k];
                }
                self.state = HdState::Free;
                None
            },
            _ => Some(HdErr::Occuped),
        }
    }
}
