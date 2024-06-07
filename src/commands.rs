use std::io::Write;

use prettytable::{format, row, Table};
use structopt::StructOpt;

use crate::path::Path;
use crate::rw::AccessMode;
use crate::utils;
use crate::vfs::{VirtualFile, VirtualFileDescription, VirtualFileSystem};

#[derive(StructOpt, Debug)]
#[structopt(name = "file system", about = "A simple file system", bin_name = "fs")]
enum Command {
    /// 列出当前目录下的文件
    Ls,

    /// 进入目录
    Cd {
        /// 目录名
        #[structopt(name = "name")]
        name: String,
    },

    /// 创建子目录
    Mkdir {
        /// 目录名
        #[structopt(name = "name")]
        name: String,
    },

    /// 删除子目录
    Rmdir {
        /// 目录名
        #[structopt(name = "name")]
        name: String,
    },

    /// 创建文件
    Create {
        /// 文件名
        #[structopt(name = "name")]
        name: String,
    },

    /// 删除文件
    Delete {
        /// 文件名
        #[structopt(name = "name")]
        name: String,
    },

    /// 退出
    Exit {
        /// 文件名
        #[structopt(name = "name")]
        name: String,
    },

    /// 打开文件
    Open {
        /// 文件名
        #[structopt(name = "name")]
        name: String,

        #[structopt(name = "mode")]
        mode: String,
    },

    /// 关闭文件
    Close {
        /// 文件名
        #[structopt(name = "name")]
        name: String,
    },

    /// 读取文件
    Read {
        /// 文件名
        #[structopt(name = "name")]
        name: String,

        /// 读取位置
        #[structopt(name = "start")]
        start: usize,

        /// 读取长度
        #[structopt(name = "len")]
        len: usize,
    },

    /// 写入文件
    Write {
        /// 文件名
        #[structopt(name = "name")]
        name: String,

        /// 写入位置
        #[structopt(name = "start")]
        start: usize,

        /// 写入数据，十六进制字符串
        #[structopt(name = "hex")]
        data: String,
    },
}


fn format_print_descriptions<D: VirtualFileDescription>(descriptions: &[D]) {
    let mut table = Table::new();

    table.set_titles(row!["名称", "类型", "大小（字节）", "创建时间", "修改时间"]);

    let mut format = *format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR;

    format.column_separator(' ');
    format.separator(
        format::LinePosition::Title,
        format::LineSeparator::new('=', '=', '=', '='),
    );
    format.separators(
        &[
            format::LinePosition::Top,
            format::LinePosition::Bottom
        ],
        format::LineSeparator::new('-', '-', '-', '-'),
    );


    table.set_format(format);

    for desc in descriptions {
        let ty_str = if desc.is_dir() {
            "目录"
        } else {
            "文件"
        };

        let size_str = if desc.is_dir() {
            "-".to_string()
        } else {
            desc.size().to_string()
        };

        let create_time_str = utils::format_time(desc.ctime() as u32);
        let modify_time_str = utils::format_time(desc.mtime() as u32);

        table.add_row(row![desc.name(), ty_str, size_str, create_time_str, modify_time_str]);
    }


    table.printstd();
}

/// 准备命令参数
fn prepare_args(mut input: String) -> Option<Vec<String>> {
    input = input.replace("\n", "")
        .replace("\r", "");

    // 解析命令
    let res = shell_words::split(&input);
    if res.is_err() {
        return None;
    }
    let mut args = res.unwrap();
    args.insert(0, "fs".to_string());

    Some(args)
}

/// 开始执行
pub fn run<FS: VirtualFileSystem>(fs: &mut FS) -> String {
    let mut path = Path::from_str("/").unwrap();
    let mut files = Vec::<FS::File>::new();

    loop {
        // 打印提示符
        print!("FS {}> ", path.to_str());
        std::io::stdout().flush().unwrap();

        // 读取输入
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();

        // 准备参数
        let args = prepare_args(input);
        if args.is_none() {
            println!("无效命令");
            continue;
        }
        let args = args.unwrap();

        if let Ok(command) = Command::from_iter_safe(args) {
            match command {
                Command::Ls => {
                    let res = fs.list(&path);
                    if res.is_err() {
                        println!("Error: {:?}", res.unwrap_err());
                        continue;
                    }
                    let res = res.unwrap();
                    format_print_descriptions(&res);
                }
                Command::Cd { name } => {
                    if name == "." {
                        continue;
                    } else if name == ".." {
                        if path.is_root() {
                            println!("当前已经是根目录");
                        } else {
                            path = path.parent().unwrap();
                        }
                        continue;
                    }

                    path.push(name);

                    let exist_res = fs.exists(&path);
                    if exist_res.is_err() {
                        println!("Error: {:?}", exist_res.unwrap_err());
                        continue;
                    }
                    let exist = exist_res.unwrap();

                    if !exist {
                        println!("不存在这个目录！");
                        path = path.parent().unwrap();
                    }
                }
                Command::Mkdir { name } => {
                    let mut new_path = path.clone();
                    new_path.push(name);

                    let mkdir_res = fs.mkdir(&new_path);
                    if mkdir_res.is_err() {
                        println!("Error: {:?}", mkdir_res.unwrap_err());
                        continue;
                    }
                    mkdir_res.unwrap()
                }
                Command::Rmdir { name } => {
                    let mut new_path = path.clone();
                    new_path.push(name);

                    let rmdir_res = fs.rmdir(&new_path);
                    if rmdir_res.is_err() {
                        println!("Error: {:?}", rmdir_res.unwrap_err());
                        continue;
                    }
                    rmdir_res.unwrap()
                }
                Command::Create { name } => {
                    let mut new_path = path.clone();
                    new_path.push(name);

                    let create_res = fs.create_file(&new_path);
                    if create_res.is_err() {
                        println!("Error: {:?}", create_res.unwrap_err());
                        continue;
                    }
                    create_res.unwrap();
                }
                Command::Delete { name } => {
                    let mut new_path = path.clone();
                    new_path.push(name);

                    let delete_res = fs.delete_file(&new_path);
                    if delete_res.is_err() {
                        println!("Error: {:?}", delete_res.unwrap_err());
                        continue;
                    }
                    delete_res.unwrap();
                }
                Command::Exit { name } => {
                    return name;
                }
                Command::Open { name, mode } => {
                    let mut new_path = path.clone();
                    new_path.push(name);

                    if files.iter()
                        .any(|file| {
                            *file.path() == new_path
                        }) {
                        println!("文件已经被打开过！");
                        continue;
                    }

                    let access_mode = if mode == "r" {
                        AccessMode::Read
                    } else if mode == "w" {
                        AccessMode::Write
                    } else if mode == "rw" || mode == "wr" {
                        AccessMode::ReadWrite
                    } else {
                        println!("需要指定文件访问模式：r、w、rw");
                        continue;
                    };

                    let open_res = fs.open(&new_path, access_mode);
                    if open_res.is_err() {
                        println!("Error: {:?}", open_res.unwrap_err());
                        continue;
                    }
                    files.push(open_res.unwrap());
                }
                Command::Close { name } => {
                    let mut new_path = path.clone();
                    new_path.push(name);

                    // 查找并拿出文件
                    let mut file = None;
                    for i in 0..files.len() {
                        if *files[i].path() == new_path {
                            file = Some(files.remove(i));
                            break;
                        }
                    }

                    if file.is_none() {
                        println!("文件没有被打开！");
                        continue;
                    }

                    let close_res = fs.close(file.unwrap());
                    if close_res.is_err() {
                        println!("Error: {:?}", close_res.unwrap_err());
                        continue;
                    }

                    close_res.unwrap();
                }
                Command::Read { name, start, len } => {
                    let mut new_path = path.clone();
                    new_path.push(name);

                    let mut file = None;
                    for i in 0..files.len() {
                        if *files[i].path() == new_path {
                            file = Some(files.get_mut(i).unwrap());
                            break;
                        }
                    }

                    if file.is_none() {
                        println!("文件没有被打开！");
                        continue;
                    }

                    let file = file.unwrap();
                    file.set_position(start);

                    let mut buf = vec![0u8; len];

                    let read_res = fs.read(file, &mut buf);
                    if read_res.is_err() {
                        println!("Error: {:?}", read_res.unwrap_err());
                        continue;
                    }
                    let read_res = read_res.unwrap();

                    println!("读取了{}字节，读取结果：{:?}", read_res, buf);
                }
                Command::Write { name, start, data } => {
                    let mut new_path = path.clone();
                    new_path.push(name);

                    let mut file = None;
                    for i in 0..files.len() {
                        if *files[i].path() == new_path {
                            file = Some(files.get_mut(i).unwrap());
                            break;
                        }
                    }

                    if file.is_none() {
                        println!("文件没有被打开！");
                        continue;
                    }

                    let file = file.unwrap();
                    file.set_position(start);

                    let data_res = hex::decode(data);
                    if data_res.is_err() {
                        println!("数据格式错误，请写入十六进制字符串！");
                        continue;
                    }
                    let data = data_res.unwrap();

                    let write_res = fs.write(file, &data);
                    if write_res.is_err() {
                        println!("Error: {:?}", write_res.unwrap_err());
                        continue;
                    }

                    let write_res = write_res.unwrap();
                    println!("写入了{}字节", write_res);
                }
            }
        } else {
            println!("无效命令");
        }
    }
}