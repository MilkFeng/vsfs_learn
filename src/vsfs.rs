use crate::{logic, utils};
use crate::logic::{ALL_INODE_RANGE, DirectoryData, DirectoryEntry};
use crate::path::Path;
use crate::repr::{DIRECT_BLOCK_COUNT, Disk, INode, SuperBlock};

const VERSION: u32 = 1;


#[derive(Debug)]
pub enum Error {
    /// 找不到路径
    PathNotFound(Path),

    /// 文件或文件夹已经存在
    FileExist(Path),

    /// 没有足够的空间
    NoSpace,
}


fn init_dir(disk: &mut Disk, inum: usize) {
    let dir_inode = unsafe { logic::get_inode_mut(&mut disk.i_blocks, inum) };
    *dir_inode = INode {
        size: 0,
        is_dir: true,
        atime: utils::time(),
        ctime: utils::time(),
        mtime: utils::time(),
        dtime: 0,
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

fn init_file(disk: &mut Disk, inum: usize) {
    let file_inode = unsafe { logic::get_inode_mut(&mut disk.i_blocks, inum) };
    *file_inode = INode {
        size: 0,
        is_dir: false,
        atime: utils::time(),
        ctime: utils::time(),
        mtime: utils::time(),
        dtime: 0,
        block_count: 0,
        block_direct: [0; DIRECT_BLOCK_COUNT],
        block_indirect: 0,
    };
}

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

pub fn get_dir(disk: &Disk, path: &Path) -> Result<DirectoryData, Error> {
    get_dir_by_path(disk, path)
        .map(|(dir, _)| dir)
        .ok_or(Error::PathNotFound(path.clone()))
}

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

pub fn exists(disk: &Disk, path: &Path) -> bool {
    get_inum_by_path(disk, path).is_some()
}

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


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_init_dir() {
        let mut disk = Disk::new();
        init(&mut disk);
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
}