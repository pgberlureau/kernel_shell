/*
    shell.rs
*/

mod fs;
use fs::Fs;
use fs::Format;
use fs::Fdesc;
use fs::FsErr;
use fs::hd::Hd;
use std::io;
use std::io::Write;

enum ParsingErr {
    NotEnoughArgs,
    TooManyArgs,
    UnknownCommand,
}
enum CmdType {
    Cat,
    Cd,
    Ls,
    Mkdir,
    Mkfs,
    Mount,
    Mv,
    Rm,
    Rmdir,
    Touch,
    Write,
    Exit,
}

fn split(input: &Format) -> Vec<Format> {
    let mut res = Vec::new();
    let mut i : usize = 0;
    let mut curr = Vec::new();

    while i < input.len() {
        if input[i] == ' ' {
            res.push(curr);
            curr = Vec::new();
        } else {
            curr.push(input[i]);
        }
        i += 1;
    }
    res.push(curr);
    res
}

struct Command {
    name: CmdType,
    args: Vec<Format>,
}

impl Command {
    fn parse(input: Format) -> Result<Command, ParsingErr> {
        let input = split(&input);
        match input[0].iter().collect::<String>().as_str() {
            //TODO deeper checks of args
            "cat" => { 
                if input.len() == 1 {
                    return Err(ParsingErr::NotEnoughArgs);
                }
                if input.len() > 2 {
                    return Err(ParsingErr::TooManyArgs);
                }
    
                Ok(Command {name: CmdType::Cat, args: input[1..].to_vec()})
            },
    
            "cd" => {
                if input.len() > 2 {
                    return Err(ParsingErr::TooManyArgs);
                }
    
                Ok(Command {name: CmdType::Cd, args: input[1..].to_vec()})
            },
    
            "ls" => {
                if input.len() > 2 {
                    return Err(ParsingErr::TooManyArgs);
                }
    
                Ok(Command {name: CmdType::Ls, args: input[1..].to_vec()})
            },
    
            "mkdir" => {
               if input.len() == 1 {
                    return Err(ParsingErr::NotEnoughArgs);
                }
                if input.len() > 2 {
                    return Err(ParsingErr::TooManyArgs);
                }
    
                Ok(Command {name: CmdType::Mkdir, args: input[1..].to_vec()})
            },
            "mkfs" => Ok(Command {name: CmdType::Mkfs, args: input[1..].to_vec()}),
            "mount" => Ok(Command {name: CmdType::Mount, args: input[1..].to_vec()}),
            "mv" => {
                if input.len() == 1 {
                    return Err(ParsingErr::NotEnoughArgs);
                }
                if input.len() > 3 {
                    return Err(ParsingErr::TooManyArgs);
                }
    
                Ok(Command {name: CmdType::Mv, args: input[1..].to_vec()})
            },
    
            "rm" => {
               if input.len() == 1 {
                    return Err(ParsingErr::NotEnoughArgs);
                }
                if input.len() > 2 {
                    return Err(ParsingErr::TooManyArgs);
                } 
    
                Ok(Command {name: CmdType::Rm, args: input[1..].to_vec()})
            },
    
            "rmdir" => {
                if input.len() == 1 {
                    return Err(ParsingErr::NotEnoughArgs);
                }
                if input.len() > 2 {
                    return Err(ParsingErr::TooManyArgs);
                }
    
    
                Ok(Command {name: CmdType::Rmdir, args: input[1..].to_vec()})
            },
    
            "touch" => {
                if input.len() == 1 {
                    return Err(ParsingErr::NotEnoughArgs);
                }
                if input.len() > 2 {
                    return Err(ParsingErr::TooManyArgs);
                }
    
                Ok(Command {name: CmdType::Touch, args: input[1..].to_vec()})
            },
    
            "write" => {
                if input.len() == 1 {
                    return Err(ParsingErr::NotEnoughArgs);
                }
                if input.len() > 3 {
                    return Err(ParsingErr::TooManyArgs);
                }
    
                Ok(Command {name: CmdType::Write, args: input[1..].to_vec()})
            },

            "exit" => {
                if input.len() == 1 {
                    return Ok(Command {name: CmdType::Exit, args: vec![]});
                }
                else {
                    return Err(ParsingErr::TooManyArgs);
                }
            },
    
            _ => Err(ParsingErr::UnknownCommand)
        }
    }
}

struct EvalResult {
    fdesc: Option<Fdesc>,
    stdout: Option<Format>,
}

fn eval(fs: &mut Fs, cmd: Command, cur: &Fdesc) -> Result<EvalResult, FsErr> {
    match cmd.name {

        CmdType::Cd => {
            let tmp : String;
            let true_args = if cmd.args.len() == 0 {"/"} else {tmp = cmd.args[0].iter().collect::<String>(); tmp.trim()};
            let res = fs.cd(cur, true_args)?;
            return Ok(EvalResult {
                fdesc: Some(res),
                stdout: None,
            })
        },

        CmdType::Ls => {
            let tmp : String;
            let true_args = if cmd.args.len() == 0 {"."} else {tmp = cmd.args[0].iter().collect::<String>(); tmp.trim()};
            let res = fs.ls(cur, true_args)?;
            return Ok(EvalResult {
                fdesc: None,
                stdout: Some(res),
            })
        },

        CmdType::Cat => {
            let tmp = cmd.args[0].iter().collect::<String>(); 
            let true_args = tmp.trim();
            let res = fs.cat(cur, true_args)?;
            return Ok(EvalResult {
                fdesc: None,
                stdout: Some(res),
            })
        },

        CmdType::Mkdir => {
            let tmp = cmd.args[0].iter().collect::<String>(); 
            let true_args = tmp.trim();
            if let Some(err) = fs.mkdir(cur, true_args) {return Err(err)};
            return Ok(EvalResult{
                fdesc: None,
                stdout: None,
            })
        },
        CmdType::Touch => {
            let tmp = cmd.args[0].iter().collect::<String>(); 
            let true_args = tmp.trim();
            if let Some(err) = fs.touch(cur, true_args) {return Err(err)};
            return Ok(EvalResult{
                fdesc: None,
                stdout: None,
            })
        },
        CmdType::Rmdir => {
            let tmp = cmd.args[0].iter().collect::<String>(); 
            let true_args = tmp.trim();
            if let Some(err) = fs.rmdir(cur, true_args) {return Err(err)};
            return Ok(EvalResult{
                fdesc: None,
                stdout: None,
            })
        },
        CmdType::Rm => {
            let tmp = cmd.args[0].iter().collect::<String>();
            let true_args = tmp.trim();
            if let Some(err) = fs.rm(cur, true_args) {return Err(err)};
            return Ok(EvalResult{
                fdesc: None,
                stdout: None,
            })
        },

        CmdType::Mkfs => {todo!();},
        CmdType::Mount => {todo!();},

        CmdType::Mv => {
            let tmp1 = cmd.args[0].iter().collect::<String>();
            let tmp2 = cmd.args[1].iter().collect::<String>();
            if let Some(err) = fs.mv(cur, tmp1.trim(), tmp2.trim()) {return Err(err)};
            return Ok(EvalResult{
                fdesc: None,
                stdout: None,
            })
        },

        CmdType::Write => {
            let tmp = cmd.args[0].iter().collect::<String>();
            if let Some(err) = fs.write(cur, tmp.trim(), &cmd.args[1]) {return Err(err)};
            return Ok(EvalResult{
                fdesc: None,
                stdout: None,
            })
        },

        CmdType::Exit => {
            return Ok(EvalResult{
                fdesc: None,
                stdout: None,
            })
        },
    }
}

fn print(data : Format){
    for k in 0..data.len(){
        print!("{}",data[k]);
    }
}

fn fmt_from(string : &str) -> Format {
    let mut fmt = Vec::new();
    let mut buff = string.chars();
    while let Some(c) = buff.next() {
        fmt.push(c);
    }
    return fmt
}

// ultra basic for the moment
fn fs_handler(err : FsErr) {
    let msg = match err {
        FsErr::HdErr(_)     => "error due to hard drive",
        FsErr::InvalidName  => "command has invalids characters",
        FsErr::FileNotFound => "file not found",
        FsErr::NoDirectory  => "this is not a directory",
        FsErr::Occuped      => "refusing to remove '.' or '..' directory",
        FsErr::ReadDir      => "cannot read a directory",
        FsErr::WriteDir     => "cannot write into a directory",
        FsErr::FileExist    => "the file already exist",
        FsErr::DirFull      => "dir is full",
        FsErr::ImapFull     => "disk is full : there is no other free inodes to write",
        FsErr::DmapFull     => "disk is full : there is no other free data blocks to write",
        FsErr::UndefBlk     => "block is undefined",
        FsErr::RemoveDir    => "cannot remove a directory",
        FsErr::InvalidCur   => "the current directory has been removed",
    };
    println!("Error : {msg}");
}

fn parsing_handler(err : ParsingErr) {
    let msg = match err {
        ParsingErr::NotEnoughArgs   => "not enough args",
        ParsingErr::TooManyArgs     => "too many args",
        ParsingErr::UnknownCommand  => "unknown command",
    };
    println!("Error : {msg}");
}

pub fn setup() {

    let mut hd = Hd::new();
    if let Some(err) = Fs::mkfs(&mut hd) {fs_handler(err)};

    let mut fs;
    loop {
        match Fs::mount(&mut hd) {
            Ok(file_system) => {fs = file_system; break},
            Err(err) => fs_handler(err),
        };
    }
    

    let mut cur_desc = fs.get_home_fdesc();

    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        let mut input = String::new();

        io::stdin()
            .read_line(&mut input)
            .expect("Error : failed to read line");
        
        let cmd = match Command::parse(fmt_from(input.as_str().trim())) {
            Ok(cmd) => cmd,
            Err(err) => {parsing_handler(err); continue}
        };

        if let CmdType::Exit = cmd.name {break};

        let result = match eval(&mut fs, cmd, &cur_desc) {
            Err(err) => {fs_handler(err); continue},
            Ok(res) => res
        };

        if let Some(fdesc) = result.fdesc {cur_desc = fdesc};
        if let Some(fmt) = result.stdout {print(fmt)};
    }
}
