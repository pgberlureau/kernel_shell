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

#[derive(Debug)]
enum ParsingErr {
    NotEnoughArgs,
    TooManyArgs,
    UnknownCommand,
    IncorrectRedirect,
    MultipleInputs,
    MultipleOutputs,
}

#[derive(Debug)]
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
    Grep,
    Echo,
    Exit,
    Empty,
}

#[derive(Debug)]
struct SimpleCommand {
    name: CmdType,
    args: Option<Vec<Format>>,
}

impl SimpleCommand {
    fn split(input: &Format) -> Vec<Format> {
        let mut res = Vec::new();
        let mut i : usize = 0;
        let mut curr = Vec::new();

        while i < input.len() {
            if input[i] == ' ' {
                if curr.len() > 0 {
                    res.push(curr);
                    curr = Vec::new();
                }
            } else {
                curr.push(input[i]);
            }
            i += 1;
        }

        if curr.len() > 0 {
            res.push(curr);
        }
        res
    }

    fn unsplit(input: &Vec<Format>) -> Format {
        let mut res = Vec::new();
        
        for w in input {
            for &c in w {
                res.push(c);
            }
            res.push(' ');
        }

        res
    }

    fn add_args(&mut self, stdin: Format) {
        let mut splitted_stdin = Self::split(&stdin);
        
        match self.args.clone() {
            Some(mut args) => {
                args.append(&mut splitted_stdin);
                self.args = Some(args)
            },
            None => {self.args = Some(splitted_stdin)}
        }
    }

    fn parse(input: Format) -> Result<SimpleCommand, ParsingErr> {
        let input = Self::split(&input);

        if input.len() == 0 {return Ok(SimpleCommand {name: CmdType::Empty, args: None});}

        match input[0].iter().collect::<String>().as_str().trim() {
            //TODO deeper checks of args
            "cat" => { 
                if input.len() > 2 {
                    return Err(ParsingErr::TooManyArgs);
                }
    
                Ok(SimpleCommand {name: CmdType::Cat, args: if input.len() == 1 {None} else {Some(input[1..].to_vec())}})
            },
    
            "cd" => {
                if input.len() > 2 {
                    return Err(ParsingErr::TooManyArgs);
                }
    
                Ok(SimpleCommand {name: CmdType::Cd, args: if input.len() == 1 {None} else {Some(input[1..].to_vec())}})
            },
    
            "ls" => {
                if input.len() > 2 {
                    return Err(ParsingErr::TooManyArgs);
                }
    
                Ok(SimpleCommand {name: CmdType::Ls, args: if input.len() == 1 {None} else {Some(input[1..].to_vec())}})
            },
    
            "mkdir" => {
                if input.len() > 2 {
                    return Err(ParsingErr::TooManyArgs);
                }
    
                Ok(SimpleCommand {name: CmdType::Mkdir, args: if input.len() == 1 {None} else {Some(input[1..].to_vec())}})
            },
            "mkfs" => Ok(SimpleCommand {name: CmdType::Mkfs, args: Some(input[1..].to_vec())}),
            "mount" => Ok(SimpleCommand {name: CmdType::Mount, args: Some(input[1..].to_vec())}),
            "mv" => {
                if input.len() == 2 {
                    return Err(ParsingErr::NotEnoughArgs);
                }

                if input.len() > 3 {
                    return Err(ParsingErr::TooManyArgs);
                }
    
                Ok(SimpleCommand {name: CmdType::Mv, args: if input.len() == 1  {None} else {Some(input[1..].to_vec())}})
            },

            "grep" => {
                if input.len() > 3 {
                    return Err(ParsingErr::TooManyArgs);
                }
    
                Ok(SimpleCommand {name: CmdType::Grep, args: if input.len() == 1  {None} else {Some(input[1..].to_vec())}})
            },
    
            "rm" => {
                if input.len() > 2 {
                    return Err(ParsingErr::TooManyArgs);
                } 
    
                Ok(SimpleCommand {name: CmdType::Rm, args: if input.len() == 1 {None} else {Some(input[1..].to_vec())}})
            },
    
            "rmdir" => {
                if input.len() > 2 {
                    return Err(ParsingErr::TooManyArgs);
                }
    
                Ok(SimpleCommand {name: CmdType::Rmdir, args: if input.len() == 1 {None} else {Some(input[1..].to_vec())}})
            },
    
            "touch" => {
                if input.len() > 2 {
                    return Err(ParsingErr::TooManyArgs);
                }
    
                Ok(SimpleCommand {name: CmdType::Touch, args: if input.len() == 1 {None} else {Some(input[1..].to_vec())}})
            },
    
            "echo" => {

                Ok(SimpleCommand {name: CmdType::Echo, args: if input.len() == 1 {None} else {Some(input[1..].to_vec())}})
            },

            "exit" => {
                if input.len() == 1 {
                    return Ok(SimpleCommand {name: CmdType::Exit, args: None});
                }
                else {
                    return Err(ParsingErr::TooManyArgs);
                }
            },

            "" => {
                return Ok(SimpleCommand {name: CmdType::Empty, args: None});
            }
    
            _ => Err(ParsingErr::UnknownCommand)
        }

    }

    fn eval(&self, fs: &mut Fs, cur: &Fdesc) -> Result<EvalResult, FsErr> {
        
        let args = match &self.args {
            Some(args) => args.clone(),
            None => vec![]
        };
        
        match self.name {
            CmdType::Cd => {
                let tmp : String;
                let true_args = if args.len() == 0 {"/"} else {tmp = args[0].iter().collect::<String>(); tmp.trim()};
                let res = fs.cd(cur, true_args)?;
                return Ok(EvalResult {
                    fdesc: Some(res),
                    stdout: None,
                    exit: false,
                })
            },
    
            CmdType::Ls => {
                let tmp : String;
                let true_args = if args.len() == 0 {"."} else {tmp = args[0].iter().collect::<String>(); tmp.trim()};
                let res = fs.ls(cur, true_args)?;
                return Ok(EvalResult {
                    fdesc: None,
                    stdout: Some(res),
                    exit: false,
                })
            },
    
            CmdType::Cat => {
                let tmp = args[0].iter().collect::<String>(); 
                let true_args = tmp.trim();
                let res = fs.cat(cur, true_args)?;
                return Ok(EvalResult {
                    fdesc: None,
                    stdout: Some(res),
                    exit: false,
                })
            },
    
            CmdType::Mkdir => {
                let tmp = args[0].iter().collect::<String>(); 
                let true_args = tmp.trim();
                if let Some(err) = fs.mkdir(cur, true_args) {return Err(err)};
                return Ok(EvalResult{
                    fdesc: None,
                    stdout: None,
                    exit: false,
                })
            },

            CmdType::Touch => {
                let tmp = args[0].iter().collect::<String>(); 
                let true_args = tmp.trim();
                if let Some(err) = fs.touch(cur, true_args) {return Err(err)};
                return Ok(EvalResult{
                    fdesc: None,
                    stdout: None,
                    exit: false,
                })
            },

            CmdType::Rmdir => {
                let tmp = args[0].iter().collect::<String>(); 
                let true_args = tmp.trim();
                if let Some(err) = fs.rmdir(cur, true_args) {return Err(err)};
                return Ok(EvalResult{
                    fdesc: None,
                    stdout: None,
                    exit: false,
                })
            },

            CmdType::Rm => {
                let tmp = args[0].iter().collect::<String>();
                let true_args = tmp.trim();
                if let Some(err) = fs.rm(cur, true_args) {return Err(err)};
                return Ok(EvalResult{
                    fdesc: None,
                    stdout: None,
                    exit: false,
                })
            },
    
            CmdType::Mkfs => {todo!();},

            CmdType::Mount => {todo!();},
    
            CmdType::Mv => {
                let tmp1 = args[0].iter().collect::<String>();
                let tmp2 = args[1].iter().collect::<String>();
                if let Some(err) = fs.mv(cur, tmp1.trim(), tmp2.trim()) {return Err(err)};
                return Ok(EvalResult{
                    fdesc: None,
                    stdout: None,
                    exit: false,
                })
            },

            CmdType::Grep => {
                let tmp1 = args[0].iter().collect::<String>();
                let tmp2 = args[1].iter().collect::<String>();
                let res = fs.grep(cur, tmp2.trim(), tmp1.trim())?;
                return Ok(EvalResult{
                    fdesc: None,
                    stdout: Some(res),
                    exit: false,
                })
            },
    
            CmdType::Echo => {//TODO
                let mut res = Self::unsplit(&args); 
                res.push('\n');
                return Ok(EvalResult{
                    fdesc: None,
                    stdout: if args.len() > 0 { Some(res) } else { None },
                    exit: false,
                })
            },
    
            CmdType::Exit => {
                return Ok(EvalResult{
                    fdesc: None,
                    stdout: None,
                    exit: true,
                })
            }

            CmdType::Empty => {
                return Ok(EvalResult{
                    fdesc: None,
                    stdout: None,
                    exit: false,
                })
            },
        }
    }
}

#[derive(Debug)]
struct Piped {
    cmd: SimpleCommand,
    output : Option<Format>,
    input  : Option<Format>
}

impl Piped {
    fn get_first_word(input: Format, offset: usize) -> (usize, usize) {
        let mut start = 0;

        while start < input.len() && input[start] == ' ' {
            start += 1;
        }

        let mut end = start;

        while end < input.len() && input[end] != ' ' {
            end += 1;
        }
        (start+offset, end+offset)
    }

    fn parse(input: Format) -> Result<Self, ParsingErr> {
        let mut inpt = None; let mut outpt = None;
        let mut start = 0;
        let mut end = 0;
        let mut simple_cmd = None;

        while end < input.len() {
            match input[end] {
                '>' => {
                    match SimpleCommand::parse(input[start..end].to_vec()) {
                        Ok(cmd) => {

                            let (wstart, wend) = Self::get_first_word(input[end+1..].to_vec(), end+1);
                            let file = input[wstart..wend].to_vec();

                            simple_cmd = Some(cmd);
                            outpt = Some(file);
                            start = wend+1;
                            end = start;
                            break;
                        },

                        Err(err) => return Err(err),
                    }
                },

                '<' => {
                    match SimpleCommand::parse(input[start..end].to_vec()) {
                        Ok(cmd) => {

                            let (wstart, wend) = Self::get_first_word(input[end+1..].to_vec(), end+1);
                            let file = input[wstart..wend].to_vec();

                            simple_cmd = Some(cmd);
                            inpt = Some(file);
                            start = wend+1;
                            end = start;
                            break;
                        },
                        Err(err) => return Err(err),
                    }
                },

                _ => end += 1,
            }
        }
        
        if let None = simple_cmd {
            match SimpleCommand::parse(input) {
                Err(err) => return Err(err),
                Ok(cmd) => return Ok(Piped {
                    cmd     : cmd,
                    output  : outpt,
                    input   : inpt
                }),
            }
        }

        while end < input.len() {
            match input[end] {
                ' ' => {start += 1; end += 1},

                '>' => {
                    if let Some(_) = outpt {return Err(ParsingErr::MultipleOutputs)}
                    let (wstart, wend) = Self::get_first_word(input[end+1..].to_vec(), end+1);
                    let file = input[wstart..wend].to_vec();
                    outpt = Some(file);
                    start = wend+1;
                    end = start;
                },

                '<' => {
                    if let Some(_) = inpt {return Err(ParsingErr::MultipleInputs);}
                    let (wstart, wend) = Self::get_first_word(input[end+1..].to_vec(), end+1);
                    let file = input[wstart..wend].to_vec();
                    inpt = Some(file);
                    start = wend+1;
                    end = start;
                },

                _ => return Err(ParsingErr::IncorrectRedirect),
            }
        }

        Ok(Piped {
            cmd     : simple_cmd.unwrap(),
            output  : outpt,
            input   : inpt
        })
    }

    fn eval(&mut self, fs: &mut Fs, cur: &Fdesc) -> Result<EvalResult, FsErr> {
        let empty_res = Ok( EvalResult {
            stdout: None,
            fdesc: None,
            exit: false,
        });

        let res = if let Some(input) = self.input.clone() {
            self.cmd.add_args(input);
            self.cmd.eval(fs,cur)?
        }
        else {self.cmd.eval(fs,cur)?};

        if let Some(output) = &self.output {
            let output_name = output.iter().collect::<String>();
            let output_name = output_name.trim();
            let mut stdout = if let Some(stdout) = res.stdout {stdout} else {vec![]};
            stdout.pop();
            match fs.write(cur, &output_name, &stdout) {
                Some(FsErr::FileNotFound) => {
                    if let Some(err) = fs.touch(cur, &output_name) {return Err(err)};
                    if let Some(err) = fs.write(cur, &output_name, &stdout) {return Err(err)};
                    return empty_res
                },
                Some(err) => return Err(err),
                None => return empty_res
            }
        }
        return Ok(res)
    }
}

type Command = Vec<Piped>;
trait Exec {
    fn parse(input: Format) -> Result<Self, ParsingErr> where Self: Sized;
    fn eval(&mut self, fs: &mut Fs, cur: &Fdesc) -> Result<EvalResult, FsErr>;
}

impl Exec for Command {
    fn parse(input: Format) -> Result<Self, ParsingErr> {
        let mut end = input.len();
        while end > 0 {
            end -= 1;
            match input[end] {
                '|' => {
                    let mut prev = Self::parse(input[..end].to_vec())?;
                    prev.push(Piped::parse(input[end+1..].to_vec())?);
                    return Ok(prev);
                }
                _ => continue,
            }
        }
        return Ok(vec![Piped::parse(input)?]);
    }

    fn eval(&mut self, fs: &mut Fs, cur: &Fdesc) -> Result<EvalResult, FsErr> {
        if self.len() == 1 {return Ok(self[0].eval(fs,cur)?)};

        let mut last = self.pop().unwrap();
        match self.eval(fs,cur)?.stdout {
            None => Ok(last.eval(fs,cur)?),
            Some(mut stdout) => {
                stdout.pop();
                if let Some(err) = fs.touch(cur,"__tmp") {return Err(err)};
                if let Some(err) = fs.write(cur,"__tmp",&stdout) {return Err(err)};
                last.cmd.add_args(vec!['_','_','t','m','p']);
                let res = Ok(last.eval(fs,cur)?);
                if let Some(err) = fs.rm(cur, "__tmp") {return Err(err)};
                res
            }
        }
    }
}

struct EvalResult {
    fdesc: Option<Fdesc>,
    stdout: Option<Format>,
    exit: bool
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
    if fmt[fmt.len()-1] == '\n' {fmt.pop();}
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
        FsErr::MvCurOrPrev  => "cannot move '.' or '..' directory"
    };
    println!("Error : {msg}");
}

fn parsing_handler(err : ParsingErr) {
    let msg = match err {
        ParsingErr::NotEnoughArgs       => "not enough args",
        ParsingErr::TooManyArgs         => "too many args",
        ParsingErr::UnknownCommand      => "unknown command",
        ParsingErr::IncorrectRedirect   => "incorrect syntax for redirect",
        ParsingErr::MultipleInputs      => "at most one input should be specifided",
        ParsingErr::MultipleOutputs     => "at most one output should be specifided",
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

    // Optionnal setup
    let mut cmd = match Command::parse(fmt_from("echo hello world pattern toto bibli ! > bar")) {
        Ok(cmd) => cmd,
        Err(err) => {parsing_handler(err);panic!("TEST SETUP FAILED !")}
    };
    if let Err(err) = cmd.eval(&mut fs, &cur_desc) {fs_handler(err);panic!("TEST SETUP FAILED !")};
    let mut cmd = match Command::parse(fmt_from("mkdir foo")) {
        Ok(cmd) => cmd,
        Err(err) => {parsing_handler(err);panic!("TEST SETUP FAILED !")}
    };
    if let Err(err) = cmd.eval(&mut fs, &cur_desc) {fs_handler(err);panic!("TEST SETUP FAILED !")};
    println!("SUCCESSFULL SETUP");
    //

    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        let mut input = String::new();

        io::stdin()
            .read_line(&mut input)
            .expect("Error : failed to read line");
        
        let mut cmd = match Command::parse(fmt_from(input.as_str())) {
            Ok(cmd) => cmd,
            Err(err) => {parsing_handler(err); continue}
        };

        let result = match cmd.eval(&mut fs, &cur_desc) {
            Err(err) => {fs_handler(err); continue},
            Ok(res) => res
        };

        if result.exit {break};

        if let Some(fdesc) = result.fdesc {cur_desc = fdesc};
        if let Some(fmt) = result.stdout {print(fmt)};
    }
}
