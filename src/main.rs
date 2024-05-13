mod shell;
use shell::setup_shell;

fn main(){
    setup_shell()
    // blablabla
}

    /*let mut hd = Hd::new();
    if let Some(err) = Fs::mkfs(&mut hd) {panic!("{:#?}",err)};

    // debug it
    /*for k in [0,1,2,3,8]{
        println!("First sector of block n°{k} : ");
        hd.display((SECT_PER_BLK as u32)*k);
    }*/

    let mut fs = match Fs::mount(&mut hd) {
        Ok(fs) => fs,
        Err(err) => panic!("{:#?}",err),
    };
    let mut cur_desc = fs.get_home_fdesc();

    let mut mkdir = |path : &str| {
        println!("> mkdir {}", path);
        if let Some(err) = fs.mkdir(&cur_desc, path) {panic!("{:#?}",err)};
    };
    mkdir("dir1/");
    mkdir("../dir2");

    let mut touch = |path : &str| {
        println!("> touch {}", path);
        if let Some(err) = fs.touch(&cur_desc, path) {panic!("{:#?}",err)};
    };
    touch("/dir1/file1");
    touch("/dir2/file2");

    let mut ls = |path : &str| {
        println!("> ls {}", path);
        match fs.ls(&cur_desc,path) {
            Err(err) => panic!("{:#?}", err),
            Ok(fmt) => print(fmt)
        }
    };
    ls("/");
    ls("/dir1");
    ls("/dir2");

    let hello = fmt_from("hello world!");
    let mut write = |path : &str, data : Format| {
        println!("> write msg in {}", path);
        if let Some(err) = fs.write(&cur_desc,path,data) {panic!("{:#?}",err)};
    };
    write("./dir1/file1",hello);

    let mut cat = |path : &str| {
        println!("> cat {}", path);
        match fs.cat(&cur_desc,path) {
            Err(err) => panic!("{:#?}", err),
            Ok(fmt) => print(fmt)
        }
    };
    cat("dir1/file1");

    let mut mv = |old_path : &str, new_path : &str| {
        println!("> mv {} {}", old_path, new_path);
        if let Some(err) = fs.mv(&cur_desc,old_path,new_path) {panic!("{:#?}",err)};
    };
    mv("dir1/file1","dir2/file3");


    let mut ls = |path : &str| {
        println!("> ls {}", path);
        match fs.ls(&cur_desc,path) {
            Err(err) => panic!("{:#?}", err),
            Ok(fmt) => print(fmt)
        }
    };
    ls("./");
    ls("dir1");
    ls("/dir2");

    let mut cat = |path : &str| {
        println!("> cat {}", path);
        match fs.cat(&cur_desc,path) {
            Err(err) => panic!("{:#?}", err),
            Ok(fmt) => print(fmt)
        }
    };
    cat("dir2/file3");

    let mut rm = |path : &str| {
        println!("> rm {}", path);
        if let Some(err) = fs.rm(&cur_desc, path) {panic!("{:#?}",err)};
    };
    rm("dir2/file2");

    let mut ls = |path : &str| {
        println!("> ls {}", path);
        match fs.ls(&cur_desc,path) {
            Err(err) => panic!("{:#?}", err),
            Ok(fmt) => print(fmt)
        }
    };
    ls("./dir2");

    let mut rmdir = |path : &str| {
        println!("> rmdir {}", path);
        if let Some(err) = fs.rmdir(&cur_desc, path) {panic!("{:#?}",err)};
    };
    rmdir("dir2");

    let mut ls = |path : &str| {
        println!("> ls {}", path);
        match fs.ls(&cur_desc,path) {
            Err(err) => panic!("{:#?}", err),
            Ok(fmt) => print(fmt)
        }
    };
    ls("./");

    let mut cd = |path : &str| {
        println!("> cd {}", path);
        match fs.cd(&cur_desc,path) {
            Err(err) => panic!("{:#?}", err),
            Ok(fd) => fd
        }
    };
    cur_desc = cd("dir1");

    let mut mkdir = |path : &str| {
        println!("> mkdir {}", path);
        if let Some(err) = fs.mkdir(&cur_desc, path) {panic!("{:#?}",err)};
    };
    mkdir("dir11/");
    mkdir("../dir21");

    let mut ls = |path : &str| {
        println!("> ls {}", path);
        match fs.ls(&cur_desc,path) {
            Err(err) => panic!("{:#?}", err),
            Ok(fmt) => print(fmt)
        }
    };
    ls("./");
    ls("..");

    let mut rmdir = |path : &str| {
        println!("> rmdir {}", path);
        if let Some(err) = fs.rmdir(&cur_desc, path) {panic!("{:#?}",err)};
    };
    rmdir("/dir1");

    let mut ls = |path : &str| {
        println!("> ls {}", path);
        match fs.ls(&cur_desc,path) {
            Err(err) => panic!("{:#?}", err),
            Ok(fmt) => print(fmt)
        }
    };
    ls(".");

    let mut cd = |path : &str| {
        println!("> cd {}", path);
        match fs.cd(&cur_desc,path) {
            Err(err) => panic!("{:#?}", err),
            Ok(fd) => fd
        }
    };
    cur_desc = cd("../");

    let mut ls = |path : &str| {
        println!("> ls {}", path);
        match fs.ls(&cur_desc,path) {
            Err(err) => panic!("{:#?}", err),
            Ok(fmt) => print(fmt)
        }
    };
    ls("/");*/