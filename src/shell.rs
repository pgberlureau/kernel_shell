mod fs;
use fs::Fs;
use fs::Format;
use fs::Fdesc;
use fs::FsErr;
use fs::hd::Hd;
use std::io;
use std::io::Write;
use std::mem::ManuallyDrop;

//Possible parsing errors
enum ParsingError {
    NotEnoughArgs,
    TooManyArgs,
    UnknownCommand,
    UnknownFlag,
}

union Error {
    fs_err: ManuallyDrop<FsErr>,
}

//Handled commands
enum CmdName {
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
}

enum Flag {
    Noflag,
}

//Type of the whole argument of a command
struct CmdArg {
    arg: Vec<char>,
}

//Type of a command, returned by the parser
struct Command {
    name: CmdName,
    flags: Vec<Flag>,
    args: Vec<Format>, //TODO switch to Vec<cmd_arg>
}

//A type of return for function eval
struct EvalResult {
    fdesc: Option<Fdesc>,
    stdout: Option<Format>,
}

const EMPTY_EVAL_RESULT : Result<EvalResult, Error> = Ok(EvalResult {
    fdesc: None,
    stdout: None,
});

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

//Parser
fn parse_command(input: Format) -> Result<Command, ParsingError> {
    let input = split(&input);
    match input[0].iter().collect::<String>().as_str() {
        //TODO deeper checks of args
        "cat" => { 
            if input.len() == 1 {
                return Err(ParsingError::NotEnoughArgs);
            }
            if input.len() > 2 {
                return Err(ParsingError::TooManyArgs);
            }

            Ok(Command {name: CmdName::Cat, flags: vec![], args: input[1..].to_vec()})
        },

        "cd" => {
            if input.len() > 2 {
                return Err(ParsingError::TooManyArgs);
            }

            Ok(Command {name: CmdName::Cd, flags: vec![], args: input[1..].to_vec()})
        },

        "ls" => {
            if input.len() > 2 {
                return Err(ParsingError::TooManyArgs);
            }

            Ok(Command {name: CmdName::Ls, flags: vec![], args: input[1..].to_vec()})
        },

        "mkdir" => {
           if input.len() == 1 {
                return Err(ParsingError::NotEnoughArgs);
            }
            if input.len() > 2 {
                return Err(ParsingError::TooManyArgs);
            }

            Ok(Command {name: CmdName::Mkdir, flags: vec![], args: input[1..].to_vec()})
        },
        "mkfs" => Ok(Command {name: CmdName::Mkfs, flags: vec![], args: input[1..].to_vec()}),
        "mount" => Ok(Command {name: CmdName::Mount, flags: vec![], args: input[1..].to_vec()}),
        "mv" => {
            if input.len() == 1 {
                return Err(ParsingError::NotEnoughArgs);
            }
            if input.len() > 3 {
                return Err(ParsingError::TooManyArgs);
            }

            Ok(Command {name: CmdName::Mv, flags: vec![], args: input[1..].to_vec()})
        },

        "rm" => {
           if input.len() == 1 {
                return Err(ParsingError::NotEnoughArgs);
            }
            if input.len() > 2 {
                return Err(ParsingError::TooManyArgs);
            } 

            Ok(Command {name: CmdName::Rm, flags: vec![], args: input[1..].to_vec()})
        },

        "rmdir" => {
            if input.len() == 1 {
                return Err(ParsingError::NotEnoughArgs);
            }
            if input.len() > 2 {
                return Err(ParsingError::TooManyArgs);
            }


            Ok(Command {name: CmdName::Rmdir, flags: vec![], args: input[1..].to_vec()})
        },

        "touch" => {
            if input.len() == 1 {
                return Err(ParsingError::NotEnoughArgs);
            }
            if input.len() > 2 {
                return Err(ParsingError::TooManyArgs);
            }

            Ok(Command {name: CmdName::Touch, flags: vec![], args: input[1..].to_vec()})
        },

        "write" => {
            if input.len() == 1 {
                return Err(ParsingError::NotEnoughArgs);
            }
            if input.len() > 3 {
                return Err(ParsingError::TooManyArgs);
            }

            Ok(Command {name: CmdName::Write, flags: vec![], args: input[1..].to_vec()})
        },

        _ => Err(ParsingError::UnknownCommand)
    }
}

//eval function
fn eval(fs: &mut Fs, cmd: Command, cur: &Fdesc) -> Result<EvalResult, Error> {
    match cmd {
        Command {name: CmdName::Cat, args, flags:_} => {
            let tmp = args[0].iter().collect::<String>(); 
            let true_args = tmp.trim();

            match fs.cat(cur, true_args) {
                Ok(res) => Ok(EvalResult {
                    fdesc: None,
                    stdout: Some(res),
                }),
                Err(err) => Err(Error {
                    fs_err: ManuallyDrop::new(err),
                }),
            }
        },

        Command {name: CmdName::Cd, args, flags: _} => {
            let tmp : String;
            let true_args = if args.len() == 0 {"/"} else {tmp = args[0].iter().collect::<String>(); tmp.trim()};

            match fs.cd(cur, true_args) {
                Ok(res) => Ok(EvalResult {
                    fdesc: Some(res),
                    stdout: None,
                }),
                Err(err) => Err(Error {
                        fs_err: ManuallyDrop::new(err),
                }),
            } 
        },

        Command {name: CmdName::Ls, args, flags: _} => {
            let tmp : String;
            let true_args = if args.len() == 0 {"."} else {tmp = args[0].iter().collect::<String>(); tmp.trim()};

            match fs.ls(cur, true_args) {
                Ok(res) => Ok(EvalResult {
                    fdesc: None,
                    stdout: Some(res),
                }),
                Err(err) => Err(Error {
                        fs_err: ManuallyDrop::new(err),
                }),
            }
        },

        Command {name: CmdName::Mkdir, args, flags: _} => {
            let tmp = args[0].iter().collect::<String>(); 
            let true_args = tmp.trim();

            if let Some(err) = fs.mkdir(cur, true_args) {
                Err(Error {
                    fs_err: ManuallyDrop::new(err),
                })
            } else {
                Ok(EvalResult{
                    fdesc: None,
                    stdout: None,
                })
            }
        },
        Command {name: CmdName::Mkfs, args:_, flags: _} => {todo!();},
        Command {name: CmdName::Mount, args:_, flags: _} => {todo!();},
        Command {name: CmdName::Mv, args, flags: _} => {
            let tmp = args[0].iter().collect::<String>();
            let tmp2 = args[1].iter().collect::<String>();
            if let Some(err) = fs.mv(cur, tmp.trim(),tmp2.trim()) {
                Err(Error {
                    fs_err: ManuallyDrop::new(err),
                })
            } else {
                Ok(EvalResult{
                    fdesc: None,
                    stdout: None,
                })
            }
        },

        Command {name: CmdName::Rm, args, flags: _} => {
            let tmp = args[0].iter().collect::<String>();
            let true_args = tmp.trim();

            if let Some(err) = fs.rm(cur, true_args) {
                Err(Error {
                    fs_err: ManuallyDrop::new(err),
                })
            } else {
                Ok(EvalResult{
                    fdesc: None,
                    stdout: None,
                })
            }
        },
        Command {name: CmdName::Rmdir, args, flags: _} => {
            let tmp = args[0].iter().collect::<String>(); 
            let true_args = tmp.trim();

            if let Some(err) = fs.rmdir(cur, true_args) {
                Err(Error {
                    fs_err: ManuallyDrop::new(err),
                })
            } else {
                Ok(EvalResult{
                    fdesc: None,
                    stdout: None,
                })
            }
        },
        Command {name: CmdName::Touch, args, flags: _} => {
            let tmp = args[0].iter().collect::<String>(); 
            let true_args = tmp.trim();

            if let Some(err) = fs.touch(cur, true_args) {
                Err(Error {
                    fs_err: ManuallyDrop::new(err),
                })
            } else {
                Ok(EvalResult{
                    fdesc: None,
                    stdout: None,
                })
            }
        },

        Command {name: CmdName::Write, args, flags: _} => {
            let tmp = args[0].iter().collect::<String>();


            if let Some(err) = fs.write(cur, tmp.trim(), &args[1]) {
                Err(Error {
                    fs_err: ManuallyDrop::new(err),
                })
            } else {
                Ok(EvalResult{
                    fdesc: None,
                    stdout: None,
                })
            }
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

pub fn setup_shell() {

    let mut hd = Hd::new();
    if let Some(err) = Fs::mkfs(&mut hd) {panic!("{:#?}",err)};

    let mut fs = match Fs::mount(&mut hd) {
        Ok(fs) => fs,
        Err(err) => panic!("{:#?}",err),
    };

    let mut cur_desc = fs.get_home_fdesc();

    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        let mut input = String::new();

        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");
        
        let cmd = match parse_command(fmt_from(input.as_str().trim())) {
            Ok(cmd) => cmd,
            Err(err) => {
                match err {
                    ParsingError::NotEnoughArgs => println!("Not enough args !"),
                    ParsingError::TooManyArgs => println!("Too many args !"),
                    ParsingError::UnknownCommand => println!("Unknown command !"),
                    ParsingError::UnknownFlag => println!("Unknown flag !"),
                }
                continue
            }
        };

        unsafe {
            match eval(&mut fs, cmd, &cur_desc) {
            Err(Error {fs_err: err}) => {
                println!("Error: {}", match ManuallyDrop::into_inner(err) {
                    FsErr::HdErr(_hd_err) => "HdErr",
                    FsErr::InvalidName => "InvalidName",
                    FsErr::FileNotFound => "FileNotFound",
                    FsErr::NoDirectory => "NoDirectory",
                    FsErr::Occuped => "Occuped",
                    FsErr::WriteDir => "WriteDir",
                    FsErr::FileExist => "FileExist",
                    FsErr::DirFull => "DirFull",
                    FsErr::ImapFull => "ImapFull",
                    FsErr::DmapFull => "DmapFull",
                    FsErr::UndefBlk => "UndefBlk",
                }
                )
            },
            Ok(EvalResult {
                fdesc: Some(fdesc),
                stdout: Some(fmt),
            }) => {cur_desc = fdesc; print(fmt)},
            Ok(EvalResult {
                fdesc: None,
                stdout: Some(fmt),
            }) => print(fmt),
            Ok(EvalResult {
                fdesc: Some(fdesc),
                stdout: None,
            }) => cur_desc = fdesc,
            Ok(EvalResult {
                fdesc: None,
                stdout: None,
            }) => continue,
        }
        }
    }
}
