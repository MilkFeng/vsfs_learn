use std::fmt::{Debug, Display, Formatter};

use crate::{logic, utils};
use crate::logic::{ALL_INODE_RANGE, DirectoryData, DirectoryEntry, get_state};
use crate::path::Path;
use crate::repr::{DIRECT_BLOCK_COUNT, Disk, INode, SuperBlock};

const VERSION: u32 = 1;


pub enum Error {
    /// 找不到路径
    PathNotFound(Path),

    /// 文件或文件夹已经存在
    FileExist(Path),

    /// 没有足够的空间
    NoSpace,

    /// 不合法的文件类型
    InvalidFileType,

    /// 文件夹不为空
    DirIsNotEmpty,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::PathNotFound(path) => write!(f, "path {} not fount", path.to_str()),
            Error::FileExist(path) => write!(f, "file {} is already exist", path.to_str()),
            Error::NoSpace => write!(f, "no space"),
            Error::InvalidFileType => write!(f, "invalid file type. file, dir, or root dir"),
            Error::DirIsNotEmpty => write!(f, "dir is not empty"),
        }
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl std::error::Error for Error {}


/// 初始化文件夹
fn init_dir(disk: &mut Disk, inum: usize) {
    let dir_inode = unsafe { logic::get_inode_mut(&mut disk.i_blocks, inum) };
    *dir_inode = INode {
        size: 0,
        is_dir: true,
        atime: utils::time(),
        ctime: utils::time(),
        mtime: utils::time(),
        block_count: 0,
        block_direct: [0; DIRECT_BLOCK_COUNT],
        block_indirect: 0,
    };

    let dir_data = DirectoryData {
        entries: vec![],
    };

    logic::write_data_struct_auto_resize(
        &mut disk.i_bitmaps,
        &mut disk.d_bitmaps,
        &mut disk.i_blocks,
        &mut disk.d_blocks,
        inum, 0, &dir_data,
    );
}

/// 初始化文件
fn init_file(disk: &mut Disk, inum: usize) {
    let file_inode = unsafe { logic::get_inode_mut(&mut disk.i_blocks, inum) };
    *file_inode = INode {
        size: 0,
        is_dir: false,
        atime: utils::time(),
        ctime: utils::time(),
        mtime: utils::time(),
        block_count: 0,
        block_direct: [0; DIRECT_BLOCK_COUNT],
        block_indirect: 0,
    };
}

/// 通过 path 获得 inode
fn get_inode_mut_by_path<'a>(disk: &'a mut Disk, path: &Path) -> Option<&'a mut INode> {
    let inum = get_inum_by_path(disk, path)?;
    Some(unsafe { logic::get_inode_mut(&mut disk.i_blocks, inum) })
}

/// 更新修改时间
pub fn update_modify_time(disk: &mut Disk, path: &Path) -> Result<(), Error> {
    let inode = get_inode_mut_by_path(disk, path);
    if let Some(inode) = inode {
        inode.mtime = utils::time();
        inode.atime = utils::time();
        Ok(())
    } else {
        Err(Error::PathNotFound(path.clone()))
    }
}

/// 更新访问时间
pub fn update_access_time(disk: &mut Disk, path: &Path) -> Result<(), Error> {
    let inode = get_inode_mut_by_path(disk, path);
    if let Some(inode) = inode {
        inode.atime = utils::time();
        Ok(())
    } else {
        Err(Error::PathNotFound(path.clone()))
    }
}

/// 初始化磁盘
pub fn init(disk: &mut Disk) {
    // 先全部置为 0
    disk.reset_zero();

    // 初始化超级块
    disk.sb = SuperBlock {
        version: VERSION,
        root_inum: 0,
    };

    // 添加根目录
    logic::set_state(&mut disk.i_bitmaps, 0, true);
    init_dir(disk, 0);
}

/// 通过 path 获得 inum
fn get_inum_by_path(disk: &Disk, path: &Path) -> Option<usize> {
    let mut inum = 0;
    for seg in path.iter() {
        let dir_data = logic::read_data_struct::<DirectoryData>(
            &disk.d_blocks,
            &disk.i_blocks,
            inum,
            0,
        );
        let target_entry = dir_data.entries.iter()
            .find(|&entry| entry.name.eq(seg));

        if let Some(target_entry) = target_entry {
            inum = target_entry.inum as usize;
        } else {
            return None;
        }
    }
    Some(inum)
}

/// 通过 path 获得 dir 和 inum
fn get_dir_by_path(disk: &Disk, path: &Path) -> Option<(DirectoryData, usize)> {
    // 找到文件夹的 inum
    let inum = get_inum_by_path(disk, path);
    if inum.is_none() {
        return None;
    }
    let inum = inum.unwrap();

    // 读取目录信息
    let dir = logic::read_data_struct::<DirectoryData>(
        &disk.d_blocks,
        &disk.i_blocks,
        inum, 0,
    );

    Some((dir, inum))
}

/// 判断这个 path 是不是目录
pub fn is_dir(disk: &Disk, path: &Path) -> Result<bool, Error> {
    let inum = get_inum_by_path(disk, path);
    if inum.is_none() {
        return Err(
            Error::PathNotFound(path.clone())
        );
    }
    let inum = inum.unwrap();

    let inode = unsafe { logic::get_inode(&disk.i_blocks, inum) };

    Ok(inode.is_dir)
}


/// 创建一个目录
pub fn create_dir(disk: &mut Disk, path: &Path, name: &str) -> Result<(), Error> {
    let (mut dir, par_inum) = get_dir_by_path(disk, path)
        .ok_or(Error::PathNotFound(path.clone()))?;

    // 检测是否存在同名文件
    if dir.exists(name) {
        let current_path = path.clone()
            .move_push(name.to_string());
        return Err(Error::FileExist(current_path));
    }

    // 创建一个 inode
    let inum = logic::get_free_item(&mut disk.i_bitmaps, ALL_INODE_RANGE)
        .ok_or(Error::NoSpace)?;

    // 初始化 inode
    logic::set_state(&mut disk.i_bitmaps, inum, true);
    init_dir(disk, inum);

    // 添加目录项
    let entry = DirectoryEntry {
        inum: inum as u32,
        name: name.to_string(),
    };
    dir.entries.push(entry);
    logic::write_data_struct_auto_resize(
        &mut disk.i_bitmaps,
        &mut disk.d_bitmaps,
        &mut disk.i_blocks,
        &mut disk.d_blocks,
        par_inum, 0, &dir,
    );

    Ok(())
}

/// 通过 path 获得目录
pub fn get_dir(disk: &Disk, path: &Path) -> Result<DirectoryData, Error> {
    if !is_dir(disk, path)? {
        return Err(Error::InvalidFileType);
    }

    get_dir_by_path(disk, path)
        .map(|(dir, _)| dir)
        .ok_or(Error::PathNotFound(path.clone()))
}

/// 文件夹是否为空
pub fn dir_is_empty(disk: &Disk, path: &Path) -> Result<bool, Error> {
    let dir = get_dir(disk, path)?;
    Ok(dir.is_empty())
}

/// 创建一个文件
pub fn create_file(disk: &mut Disk, path: &Path, name: &str) -> Result<(), Error> {
    let (mut dir, par_inum) = get_dir_by_path(disk, path)
        .ok_or(Error::PathNotFound(path.clone()))?;

    // 检测是否存在同名文件
    if dir.exists(name) {
        let current_path = path.clone()
            .move_push(name.to_string());
        return Err(Error::FileExist(current_path));
    }

    // 创建一个 inode
    let inum = logic::get_free_item(&mut disk.i_bitmaps, ALL_INODE_RANGE)
        .ok_or(Error::NoSpace)?;

    // 初始化 inode
    logic::set_state(&mut disk.i_bitmaps, inum, true);
    init_file(disk, inum);

    // 添加目录项
    let entry = DirectoryEntry {
        inum: inum as u32,
        name: name.to_string(),
    };
    dir.entries.push(entry);
    logic::write_data_struct_auto_resize(
        &mut disk.i_bitmaps,
        &mut disk.d_bitmaps,
        &mut disk.i_blocks,
        &mut disk.d_blocks,
        par_inum, 0, &dir,
    );

    Ok(())
}

/// 某个文件或目录是否存在
pub fn exists(disk: &Disk, path: &Path) -> bool {
    get_inum_by_path(disk, path).is_some()
}

/// 读文件
pub fn read_file(disk: &Disk, path: &Path, start_pos: usize, buf: &mut [u8]) -> Result<(), Error> {
    let inum = get_inum_by_path(disk, path)
        .ok_or(Error::PathNotFound(path.clone()))?;

    let inode = unsafe { logic::get_inode(&disk.i_blocks, inum) };
    if inode.is_dir {
        return Err(Error::PathNotFound(path.clone()));
    }

    logic::read_data(&disk.d_blocks, &disk.i_blocks, inum, start_pos, buf);

    Ok(())
}

/// 写文件
pub fn write_file(disk: &mut Disk, path: &Path, start_pos: usize, buf: &[u8]) -> Result<(), Error> {
    let inum = get_inum_by_path(disk, path)
        .ok_or(Error::PathNotFound(path.clone()))?;

    let inode = unsafe { logic::get_inode(&disk.i_blocks, inum) };
    if inode.is_dir {
        return Err(Error::PathNotFound(path.clone()));
    }

    logic::write_data_auto_resize(
        &mut disk.i_bitmaps,
        &mut disk.d_bitmaps,
        &mut disk.i_blocks,
        &mut disk.d_blocks,
        inum, start_pos, buf,
    );

    Ok(())
}

/// 通过 path 获得 inode
pub fn get_inode_by_path<'a>(disk: &'a Disk, path: &Path) -> Option<&'a INode> {
    let inum = get_inum_by_path(disk, path)?;
    Some(unsafe { logic::get_inode(&disk.i_blocks, inum) })
}

/// 更新目录数据，删掉一些已经被 free 的文件
fn update_dir_data(disk: &mut Disk, path: &Path) -> Result<(), Error> {
    let (dir, inum) = get_dir_by_path(disk, path)
        .ok_or(Error::PathNotFound(path.clone()))?;
    let filtered_entries = dir.entries.into_iter()
        .filter(|entry| {
            get_state(
                &mut disk.i_bitmaps,
                entry.inum as usize,
            )
        })
        .collect::<Vec<_>>();
    let dir = DirectoryData {
        entries: filtered_entries
    };

    logic::write_data_struct_auto_resize(
        &mut disk.i_bitmaps,
        &mut disk.d_bitmaps,
        &mut disk.i_blocks,
        &mut disk.d_blocks,
        inum, 0, &dir,
    );

    Ok(())
}

/// 删除文件
pub fn delete_file(disk: &mut Disk, path: &Path) -> Result<(), Error> {
    if is_dir(disk, path)? {
        return Err(Error::InvalidFileType);
    }

    let inum = get_inum_by_path(disk, path)
        .ok_or(Error::PathNotFound(path.clone()))?;

    logic::free_inode(
        &mut disk.i_bitmaps,
        &mut disk.d_bitmaps,
        &mut disk.i_blocks,
        inum,
    );

    let parent = path.clone().parent()
        .ok_or(Error::InvalidFileType)?;

    update_dir_data(disk, &parent)
}

/// 删除文件夹
pub fn delete_dir(disk: &mut Disk, path: &Path) -> Result<(), Error> {
    if !is_dir(disk, path)? {
        return Err(Error::InvalidFileType);
    }

    let (dir, inum) = get_dir_by_path(disk, path)
        .ok_or(Error::PathNotFound(path.clone()))?;

    // 根目录不能删除
    if inum == 0 {
        return Err(Error::InvalidFileType);
    }

    // 看一下文件夹是否为空
    if !dir.entries.is_empty() {
        return Err(Error::DirIsNotEmpty);
    }

    logic::free_inode(
        &mut disk.i_bitmaps,
        &mut disk.d_bitmaps,
        &mut disk.i_blocks,
        inum,
    );

    let parent = path.clone().parent()
        .ok_or(Error::InvalidFileType)?;

    update_dir_data(disk, &parent)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_init_dir() {
        let mut disk = Disk::new();
        init(&mut disk);
        logic::set_state(&mut disk.i_bitmaps, 1, true);
        init_dir(&mut disk, 1);

        let inode = unsafe { logic::get_inode(&disk.i_blocks, 0) };

        assert_ne!(inode.size, 0);
    }

    #[test]
    fn test_create_dir() {
        let mut disk = Disk::new();
        init(&mut disk);
        let mut path = Path::root();
        create_dir(&mut disk, &path, "test").unwrap();

        let dir = get_dir(&disk, &path).unwrap();
        assert_eq!(dir.entries.len(), 1);
        assert_eq!(dir.entries[0].name, "test");
        assert_eq!(is_dir(&disk, &path).unwrap(), true);

        path.push("test".to_string());
        let dir = get_dir(&disk, &path).unwrap();
        assert_eq!(dir.entries.len(), 0);
        assert_eq!(is_dir(&disk, &path).unwrap(), true);
        println!("{:?}", dir);
    }

    #[test]
    fn test_create_file() {
        let mut disk = Disk::new();
        init(&mut disk);
        let mut path = Path::root();
        create_file(&mut disk, &path, "test.c").unwrap();

        let dir = get_dir(&disk, &path).unwrap();
        assert_eq!(dir.entries.len(), 1);
        assert_eq!(dir.entries[0].name, "test.c");

        path.push("test.c".to_string());
        assert_eq!(is_dir(&disk, &path).unwrap(), false);
    }

    #[test]
    fn test_rw_file() {
        let mut disk = Disk::new();
        init(&mut disk);
        let mut path = Path::root();
        create_file(&mut disk, &path, "test.c").unwrap();

        path.push("test.c".to_string());
        let mut buf = [0; 4096];
        for i in 0..4096 {
            buf[i] = i as u8;
        }
        write_file(&mut disk, &path, 1000, &buf).unwrap();

        let mut read_buf = [0; 4096];
        read_file(&disk, &path, 1000, &mut read_buf).unwrap();
        assert_eq!(read_buf, buf);
    }

    #[test]
    fn test_delete() {
        let mut disk = Disk::new();
        init(&mut disk);
        let mut path = Path::root();
        create_file(&mut disk, &path, "test.c").unwrap();

        let dir = get_dir(&disk, &path).unwrap();
        assert_eq!(dir.entries.len(), 1);
        assert_eq!(dir.entries[0].name, "test.c");

        path.push("test.c".to_string());
        delete_file(&mut disk, &path).unwrap();

        path = path.parent().unwrap();

        let dir = get_dir(&disk, &path).unwrap();
        assert_eq!(dir.entries.len(), 0);
    }

    #[should_panic]
    #[test]
    fn test_delete_dir_panic() {
        let mut disk = Disk::new();
        init(&mut disk);

        let mut path = Path::root();
        create_file(&mut disk, &path, "test.c").unwrap();
        create_dir(&mut disk, &path, "test1").unwrap();

        let dir = get_dir(&disk, &path).unwrap();
        assert_eq!(dir.entries.len(), 2);
        assert_eq!(dir.entries[0].name, "test.c");
        assert_eq!(dir.entries[1].name, "test1");

        let mut path = Path::from_str("/test1").unwrap();
        create_file(&mut disk, &path, "test2.c").unwrap();
        create_dir(&mut disk, &path, "test4").unwrap();

        let dir = get_dir(&disk, &path).unwrap();
        assert_eq!(dir.entries.len(), 2);
        assert_eq!(dir.entries[0].name, "test2.c");
        assert_eq!(dir.entries[1].name, "test4");

        delete_dir(&mut disk, &path).unwrap();
    }

    #[should_panic]
    #[test]
    fn test_delete_dir_panic_2() {
        let mut disk = Disk::new();
        init(&mut disk);
        delete_dir(&mut disk, &Path::root()).unwrap();
    }

    #[test]
    fn test_delete_2() {
        let mut disk = Disk::new();
        init(&mut disk);


        let mut path = Path::root();
        create_file(&mut disk, &path, "test.c").unwrap();
        create_dir(&mut disk, &path, "test1").unwrap();
        create_dir(&mut disk, &path, "test2").unwrap();
        create_dir(&mut disk, &path, "test3").unwrap();

        let dir = get_dir(&disk, &path).unwrap();
        assert_eq!(dir.entries.len(), 4);
        assert_eq!(dir.entries[0].name, "test.c");
        assert_eq!(dir.entries[1].name, "test1");
        assert_eq!(dir.entries[2].name, "test2");
        assert_eq!(dir.entries[3].name, "test3");


        assert_eq!(get_dir(&disk, &Path::from_str("/test1").unwrap()).unwrap().len(), 0);
        assert_eq!(get_dir(&disk, &Path::from_str("/test2").unwrap()).unwrap().len(), 0);
        assert_eq!(get_dir(&disk, &Path::from_str("/test3").unwrap()).unwrap().len(), 0);

        let mut path = Path::from_str("/test1").unwrap();
        create_file(&mut disk, &path, "test2.c").unwrap();
        create_dir(&mut disk, &path, "test4").unwrap();

        let dir = get_dir(&disk, &path).unwrap();
        assert_eq!(dir.entries.len(), 2);
        assert_eq!(dir.entries[0].name, "test2.c");
        assert_eq!(dir.entries[1].name, "test4");

        assert_eq!(get_dir(&disk, &Path::from_str("/test1").unwrap()).unwrap().len(), 2);
        assert_eq!(get_dir(&disk, &Path::from_str("/test2").unwrap()).unwrap().len(), 0);
        assert_eq!(get_dir(&disk, &Path::from_str("/test3").unwrap()).unwrap().len(), 0);


        delete_dir(&mut disk, &Path::from_str("/test2").unwrap()).unwrap();

        let dir = get_dir(&disk, &Path::root()).unwrap();
        assert_eq!(dir.entries.len(), 3);
        assert_eq!(dir.entries[0].name, "test.c");
        assert_eq!(dir.entries[1].name, "test1");
        assert_eq!(dir.entries[2].name, "test3");


        delete_file(&mut disk, &Path::from_str("/test.c").unwrap()).unwrap();

        let dir = get_dir(&disk, &Path::root()).unwrap();
        assert_eq!(dir.entries.len(), 2);
        assert_eq!(dir.entries[0].name, "test1");
        assert_eq!(dir.entries[1].name, "test3");

        delete_file(&mut disk, &Path::from_str("/test1/test2.c").unwrap()).unwrap();

        let dir = get_dir(&disk, &Path::from_str("/test1").unwrap()).unwrap();
        assert_eq!(dir.entries.len(), 1);
        assert_eq!(dir.entries[0].name, "test4");
    }
}