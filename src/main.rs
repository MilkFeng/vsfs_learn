use structopt::StructOpt;

use crate::io::{Loadable, Savable};
use crate::vfs::VirtualFileSystem;

mod io;
mod repr;
mod logic;
mod rw;
mod vsfs;
mod path;
mod utils;
mod vfs;
mod vsfs_vfs;
mod commands;


#[derive(StructOpt, Debug)]
#[structopt(name = "file system", about = "A simple file system")]
enum Command {
    /// 创建一个新的文件系统并加载
    New {},

    /// 加载已有的文件系统
    Sfs {
        /// 文件系统文件路径
        #[structopt(name = "path")]
        path: std::path::PathBuf
    },

    /// 显示帮助信息
    Help,
}


fn main() {
    let command = Command::from_args();
    match command {
        Command::New {} => {
            println!("准备创建文件系统...");

            let mut disk = repr::Disk::new();
            let mut fs = vsfs_vfs::VerySimpleFileSystem::new(&mut disk);
            let res = fs.init();

            if res.is_err() {
                println!("文件系统创建失败！");
                return;
            }

            println!("文件系统创建成功！");

            let name = commands::run(&mut fs);

            println!("文件系统退出，准备将文件系统保存到: {:?}", name);
            disk.save(name).unwrap();
            println!("文件系统保存成功！");
        }
        Command::Sfs { path } => {
            println!("准备加载文件系统: {:?}", path);
            let mut disk = repr::Disk::load(path).unwrap();
            let mut fs = vsfs_vfs::VerySimpleFileSystem::new(&mut disk);
            println!("文件系统加载成功！");

            let name = commands::run(&mut fs);

            println!("文件系统退出，准备将文件系统保存到: {:?}", name);
            disk.save(name).unwrap();
            println!("文件系统保存成功！");
        },
        Command::Help => {
            print!("\n");
            Command::clap().print_help().unwrap();
            print!("\n\n");
        }
    }
}
