use std::error::Error;
use std::fmt::Debug;
use std::fs::File;
use std::io::{Read, Write};
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::path::Path;

use crate::io::{Loadable, Savable};

/// 128 MB
pub const SIZE: usize = 4096 * 16;
/// 8 K 个 IBlock，256 K 个 INode
pub const INDEX_BLOCK_COUNT: usize = 1024 * 4;
/// 4 个 IBlock 位图块
pub const INDEX_BITMAP_BLOCK_COUNT: usize = INDEX_BLOCK_COUNT / 1024;
/// DataBlock 位图块个数 (DATA_BLOCK_COUNT / 1024 / 32 向上取整)
pub const DATA_BITMAP_BLOCK_COUNT: usize = 2;
/// 数据块个数
pub const DATA_BLOCK_COUNT: usize = SIZE - 1 - INDEX_BLOCK_COUNT - INDEX_BITMAP_BLOCK_COUNT - DATA_BITMAP_BLOCK_COUNT;
/// 按照 4096 对齐
pub const ALIGN_SIZE: usize = 4096;
/// 一个 inode 可以存放 12 个直接块，一个间接块
pub const DIRECT_BLOCK_COUNT: usize = 12;

/// 磁盘结构
#[repr(align(4096))]
#[derive(PartialEq)]
pub struct Disk {
    pub sb: SuperBlock,                                         // 超级块

    pub i_bitmaps: [BitmapBlock; INDEX_BITMAP_BLOCK_COUNT],     // inode 位图
    pub d_bitmaps: [BitmapBlock; DATA_BITMAP_BLOCK_COUNT],      // 数据块位图

    pub i_blocks: [IBlock; INDEX_BLOCK_COUNT],                  // inode 块
    pub d_blocks: [DataBlock; DATA_BLOCK_COUNT],                // 数据块
}

impl Debug for Disk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Disk")
            .field("sb", &self.sb)
            .finish()
    }
}


/// inode 结构
#[repr(align(128))]
#[derive(PartialEq, Eq, Hash, Debug, Copy, Clone)]
pub struct INode {
    pub size: u32,                                      // 文件大小
    pub is_dir: bool,                                   // 是否是目录

    pub atime: u32,                                     // 文件最近一次被访问的时间
    pub ctime: u32,                                     // 文件的创建时间
    pub mtime: u32,                                     // 文件最近一次被修改的时间
    pub dtime: u32,                                     // 这个 inode 被删除的时间

    pub block_count: u32,                               // 这个 inode 占用的块数（包括直接块和间接块）
    pub block_direct: [u32; DIRECT_BLOCK_COUNT],        // 直接块，存放数据块编号
    pub block_indirect: u32,                            // 一级间接块，属于索引块
}

/// inode 块，一个块可以存放 32 个 inode
#[repr(align(4096))]
pub union IBlock {
    pub inodes: ManuallyDrop<[INode; 32]>,              // 索引块，一个块可以存放 32 个 inode
    pub idx: ManuallyDrop<[u32; 1024]>,                 // 一级间接块，那么可以存放 1024 个索引
}

impl PartialEq for IBlock {
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            self.idx == other.idx
        }
    }
}


/// 位图块
#[repr(align(4096))]
#[derive(PartialEq)]
pub struct BitmapBlock {
    pub bitmaps: [u32; 1024],           // 可以表示 32 * 1024 = 32768 个状态；1024 个 IBlock 或者 32768 个 DataBlock
}


/// 超级块
#[repr(align(4096))]
#[derive(PartialEq, Debug)]
pub struct SuperBlock {
    pub version: u32,                   // 文件系统版本
    pub root_inum: u32,                 // 根目录的 inode 编号
}


/// 数据块
#[repr(align(4096))]
#[derive(PartialEq)]
pub struct DataBlock {
    pub data: [u8; 4096],               // 数据，一个块 4096 字节
}



impl Disk {
    pub fn new() -> Box<Disk> {
        unsafe {
            // 直接从堆栈分配内存
            let size = std::mem::size_of::<Disk>();
            let disk = std::alloc::alloc(std::alloc::Layout::from_size_align(size, 4096).unwrap()) as *mut Disk;
            Box::from_raw(disk)
        }
    }

    pub fn to_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(self as *const Disk as *const u8, std::mem::size_of::<Disk>())
        }
    }

    pub fn reset_zero(&mut self) {
        unsafe {
            std::ptr::write_bytes(self as *mut Disk as *mut u8, 0, std::mem::size_of::<Disk>());
        }
    }
}


impl Loadable for Disk {
    fn load<P: AsRef<Path>>(path: P) -> Result<Box<Self>, impl Error> {
        let mut file = File::open(path)?;
        let mut bytes = vec![0u8; std::mem::size_of::<Disk>()];
        file.read(&mut bytes)?;

        let disk = Disk::new();
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), disk.as_ref() as *const Disk as *mut u8, bytes.len());
        }


        Ok::<Box<Self>, std::io::Error>(disk)
    }
}

impl Savable for Disk {
    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), impl Error> {
        let mut file = File::create(path)?;
        let bytes = self.to_bytes();

        file.write_all(&bytes)?;
        Ok::<(), std::io::Error>(())
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_size() {
        assert_eq!(std::mem::size_of::<INode>(), 128);

        assert_eq!(std::mem::size_of::<IBlock>(), 4096);
        assert_eq!(std::mem::size_of::<BitmapBlock>(), 4096);
        assert_eq!(std::mem::size_of::<SuperBlock>(), 4096);

        let size = std::mem::size_of::<Disk>();
        println!("磁盘大小：{}", size);
        println!("最大存储空间：{} 字节", DATA_BLOCK_COUNT * 4096);

        println!("{:?}", DATA_BITMAP_BLOCK_COUNT);
    }

    #[test]
    fn test_new() {
        let disk = Disk::new();
        assert_eq!(disk.sb.version, 0);
        assert_eq!(disk.sb.root_inum, 0);
    }

    #[test]
    fn test_save_load() {
        let mut disk = Disk::new();
        disk.sb.version = 24;
        disk.sb.root_inum = 3333;
        disk.save("disk").unwrap();


        let new_disk = Disk::load("disk").unwrap();
        assert_eq!(new_disk, disk);

        // 删除文件
        std::fs::remove_file("disk").unwrap();
    }
}