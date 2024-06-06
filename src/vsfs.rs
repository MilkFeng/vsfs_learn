use crate::{logic, utils};
use crate::logic::DirectoryData;
use crate::path::Path;
use crate::repr::{DIRECT_BLOCK_COUNT, Disk, INode, SuperBlock};

const VERSION: u32 = 1;


#[derive(Debug)]
pub enum Error {
    /// 找不到路径
    PathNotFound(Path),

    /// 文件或文件夹已经存在
    FileExist(Path),
}



fn init_dir(disk: &mut Disk, inum: usize) {
    let dir_inode = unsafe { logic::get_inode_mut(&mut disk.i_blocks, inum) };
    *dir_inode = INode {
        size: 0,
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

fn get_dir_by_path(disk: &Disk, path: &Path) -> Option<DirectoryData> {
    // 找到父文件夹的 inum
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

    Some(dir)
}

pub fn create_dir(disk: &mut Disk, path: &Path, name: &str) -> Result<(), Error> {
    let dir = get_dir_by_path(disk, path);
    if dir.is_none() {
        return Err(
            Error::PathNotFound(path.clone())
        );
    }
    let dir = dir.unwrap();

    // 检测是否存在同名文件
    if dir.entries.iter()
        .any(|entry| entry.name.eq(name)) {

        let mut current_path = path.clone();
        current_path.push(name.to_string());
        return Err(
            Error::FileExist(current_path)
        );
    }

    // 创建一个 inode
    // let inum = logic::get_free_item(&mut disk.i_bitmaps, ALL_INDEX_COUNT);


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
        let path = Path::from_str("/test").unwrap();
        // create_dir(&mut disk, &path).unwrap();
    }
}