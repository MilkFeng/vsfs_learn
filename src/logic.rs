use std::ops::{Deref, DerefMut, Range};

use serde::{Deserialize, Serialize};

use crate::repr::*;

#[derive(Serialize, Deserialize)]
#[derive(PartialEq, Eq, Debug, Hash, Clone)]
pub struct DirectoryEntry {
    pub name: String,
    pub inum: u32,
}


#[derive(Serialize, Deserialize)]
#[derive(PartialEq, Eq, Debug, Hash, Clone)]
pub struct DirectoryData {
    pub entries: Vec<DirectoryEntry>,
}

impl DirectoryData {
    pub fn exists(&self, name: &str) -> bool {
        self.entries.iter()
            .any(|entry| entry.name == name)
    }
}

impl Deref for DirectoryData {
    type Target = Vec<DirectoryEntry>;

    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}

impl DerefMut for DirectoryData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entries
    }
}

/// 获取 bitmap 状态，true 表示已经被占用，false 表示空闲
pub fn get_state(bitmap_blocks: &[BitmapBlock], index: usize) -> bool {
    let block_index = index / (ALIGN_SIZE * 32);
    let u32_index = index % (ALIGN_SIZE * 32) / 32;
    let bit_index = index % 32;

    let block = &bitmap_blocks[block_index];
    let u32 = block.bitmaps[u32_index];
    u32 & (1 << bit_index) != 0
}

/// 获取一个块的状态，在一级间接索引块使用
pub fn get_block_state(bitmap_blocks: &[BitmapBlock], index: usize) -> u32 {
    let block_index = index / ALIGN_SIZE;
    let u32_index = index % ALIGN_SIZE;

    let block = &bitmap_blocks[block_index];
    let u32 = &block.bitmaps[u32_index];

    *u32
}

/// 设置 bitmap 状态，true 表示已经被占用，false 表示空闲
pub fn set_state(bitmap_blocks: &mut [BitmapBlock], index: usize, state: bool) {
    let block_index = index / (ALIGN_SIZE * 32);
    let u32_index = index % (ALIGN_SIZE * 32) / 32;
    let bit_index = index % 32;

    let block = &mut bitmap_blocks[block_index];
    let u32 = &mut block.bitmaps[u32_index];
    if state {
        *u32 |= 1 << bit_index;
    } else {
        *u32 &= !(1 << bit_index);
    }
}

/// 设置一个块的状态，在一级间接索引块使用
pub unsafe fn set_block_state(bitmap_blocks: &mut [BitmapBlock], index: usize, state: bool) {
    let block_index = index / ALIGN_SIZE;
    let u32_index = index % ALIGN_SIZE;

    let block = &mut bitmap_blocks[block_index];
    let u32 = &mut block.bitmaps[u32_index];

    if state {
        *u32 = u32::MAX
    } else {
        *u32 = 0;
    }
}

/// 获得 bitmap 中的空闲项
pub fn get_free_item(bitmap_blocks: &[BitmapBlock], range: Range<usize>) -> Option<usize> {
    for i in range {
        if !get_state(bitmap_blocks, i) {
            return Some(i);
        }
    }
    None
}

/// 获得一个空闲块，32 个项
pub fn get_free_block(bitmap_blocks: &[BitmapBlock], range: Range<usize>) -> Option<usize> {
    for i in range {
        if get_block_state(bitmap_blocks, i) == 0 {
            return Some(i);
        }
    }
    None
}

/// 所有索引块的范围
pub const ALL_INODE_RANGE: Range<usize> = 0..INDEX_BLOCK_COUNT * 32;

/// 所有的索引块范围
pub const ALL_INDEX_BLOCK_RANGE: Range<usize> = 0..INDEX_BLOCK_COUNT;

/// 所有数据块的范围
pub const ALL_DATA_BLOCK_RANGE: Range<usize> = 0..DATA_BLOCK_COUNT;


/// 获取 inode
pub unsafe fn get_inode(inode_blocks: &[IBlock], inum: usize) -> &INode {
    let block_index = inum / 32;
    let inode_index = inum % 32;
    &inode_blocks[block_index].inodes[inode_index]
}

/// 获取可变 inode
pub unsafe fn get_inode_mut(inode_blocks: &mut [IBlock], inum: usize) -> &mut INode {
    let block_index = inum / 32;
    let inode_index = inum % 32;
    let ptr = &mut inode_blocks[block_index] as *mut IBlock as *mut [INode; 32];
    &mut (*ptr)[inode_index]
}

/// 在 inode_blocks 中根据 inum 获取一级间接块，然后根据 index 获取 dnum
pub unsafe fn get_indirect_dnum(inode_blocks: &[IBlock], inum: usize, index: usize) -> &u32 {
    &inode_blocks[inum].idx[index]
}

/// 在 inode_blocks 中根据 inum 获取一级间接块，然后根据 index 获取 dnum
pub unsafe fn get_indirect_dnum_mut(inode_blocks: &mut [IBlock], inum: usize, index: usize) -> &mut u32 {
    let ptr = &mut inode_blocks[inum] as *mut IBlock as *mut[u32; 1024];
    &mut (*ptr)[index]
}

/// 获取一级间接块
pub unsafe fn get_indirect_block(inode_blocks: &[IBlock], inum: usize) -> &[u32; 1024] {
    &inode_blocks[inum].idx
}

/// 获取一级间接块
pub unsafe fn get_indirect_block_mut(inode_blocks: &mut [IBlock], inum: usize) -> &mut [u32; 1024] {
    let ptr = &mut inode_blocks[inum] as *mut IBlock as *mut[u32; 1024];
    &mut *ptr
}

/// 获取 data block
pub fn get_data_block(data_blocks: &[DataBlock], dnum: usize) -> &[u8] {
    &data_blocks[dnum].data
}

/// 获取可变 data block
pub fn get_data_block_mut(data_blocks: &mut [DataBlock], dnum: usize) -> &mut [u8] {
    &mut data_blocks[dnum].data
}

/// 在 inode_blocks 中根据 inode 中的信息获取 index 对应的编号
pub fn get_dnum(index_blocks: &[IBlock], inum: usize, index: usize) -> &u32 {
    let inode = unsafe { get_inode(index_blocks, inum) };
    if index >= inode.block_count as usize {
        panic!("index out of range")
    }
    if index < DIRECT_BLOCK_COUNT {
        &inode.block_direct[index]
    } else if index < DIRECT_BLOCK_COUNT + 1024 {
        let index_data_block = inode.block_indirect as usize;
        unsafe { get_indirect_dnum(index_blocks, index_data_block, index - DIRECT_BLOCK_COUNT) }
    } else {
        panic!("index out of range")
    }
}

/// 在 inode_blocks 中根据 inode 中的信息获取 index 对应的可变编号
pub fn get_dnum_mut(index_blocks: &mut [IBlock], inum: usize, index: usize) -> &mut u32 {
    if index < DIRECT_BLOCK_COUNT {
        let inode = unsafe { get_inode_mut(index_blocks, inum) };
        if index >= inode.block_count as usize {
            panic!("index out of range")
        }
        &mut inode.block_direct[index]
    } else if index < DIRECT_BLOCK_COUNT + 1024 {
        let inode = unsafe { get_inode(index_blocks, inum) };
        let index_data_block = inode.block_indirect as usize;
        if index >= inode.block_count as usize {
            panic!("index out of range")
        }
        unsafe { get_indirect_dnum_mut(index_blocks, index_data_block, index - DIRECT_BLOCK_COUNT) }
    } else {
        panic!("index out of range")
    }
}

/// 扩充数据块
pub fn extend_data_block_of_inode(
    index_bitmap_blocks: &mut [BitmapBlock],
    data_bitmap_blocks: &mut [BitmapBlock],
    index_blocks: &mut [IBlock],
    inum: usize,
    count: usize
) {
    let inode = unsafe { get_inode(index_blocks, inum) };
    if inode.block_count as usize >= count {
        return;
    }

    if count <= DIRECT_BLOCK_COUNT {
        let inode = unsafe { get_inode_mut(index_blocks, inum) };
        // 直接
        for i in inode.block_count as usize..count {
            let dnum = get_free_item(data_bitmap_blocks, ALL_DATA_BLOCK_RANGE).unwrap();
            set_state(data_bitmap_blocks, dnum, true);
            inode.block_direct[i] = dnum as u32;
        }
        inode.block_count = count as u32;
    } else {

        let inode = unsafe { get_inode(index_blocks, inum) }.clone();
        if inode.block_count < DIRECT_BLOCK_COUNT as u32 {
            // 直接的扩充到 DIRECT_BLOCK_COUNT
            extend_data_block_of_inode(
                index_bitmap_blocks,
                data_bitmap_blocks,
                index_blocks,
                inum, DIRECT_BLOCK_COUNT
            );

            // 申请一个间接块
            let block_id = get_free_block(index_bitmap_blocks, ALL_INDEX_BLOCK_RANGE).unwrap();
            unsafe { set_block_state(index_bitmap_blocks, block_id, true); }

            // 设置间接块
            let inode = unsafe { get_inode_mut(index_blocks, inum) };
            inode.block_indirect = block_id as u32 * 32;
        }

        let inode = unsafe { get_inode(index_blocks, inum) }.clone();
        for i in inode.block_count as usize..count {
            let dnum = get_free_item(data_bitmap_blocks, ALL_DATA_BLOCK_RANGE).unwrap();
            let target_dnum = unsafe {
                get_indirect_dnum_mut(index_blocks, inode.block_indirect as usize, i - DIRECT_BLOCK_COUNT)
            };
            set_state(data_bitmap_blocks, dnum, true);
            *target_dnum = dnum as u32;
        }
        let inode = unsafe { get_inode_mut(index_blocks, inum) };
        inode.block_count = count as u32;
        return;
    }
}

/// 缩减数据块
pub fn shrink_data_block_of_inode(
    index_bitmap_blocks: &mut [BitmapBlock],
    data_bitmap_blocks: &mut [BitmapBlock],
    index_blocks: &mut [IBlock],
    inum: usize,
    count: usize
) {
    let inode = unsafe { get_inode(index_blocks, inum) }.clone();
    if inode.block_count as usize <= count {
        return;
    }

    // 先释放掉数据块
    for i in count..inode.block_count as usize {
        let dnum = get_dnum_mut(index_blocks, inum, i);
        set_state(data_bitmap_blocks, *dnum as usize, false);
        *dnum = 0;
    }

    // 如果之前存在 indirect，之后不存在，则释放掉 indirect
    if count <= DIRECT_BLOCK_COUNT && inode.block_count as usize > DIRECT_BLOCK_COUNT {
        let state = get_block_state(
            index_bitmap_blocks,
            inode.block_indirect as usize
        );
        if state != u32::MAX && state != 0 {
            panic!("block state panic!");
        }

        unsafe {
            set_block_state(
                index_bitmap_blocks,
                inode.block_indirect as usize,
                false
            );
        }

        let inode = unsafe { get_inode_mut(index_blocks, inum) };
        inode.block_indirect = 0;
    }

    let inode = unsafe { get_inode_mut(index_blocks, inum) };
    inode.block_count = count as u32;
}

pub fn resize_data_block_of_inode(
    index_bitmap_blocks: &mut [BitmapBlock],
    data_bitmap_blocks: &mut [BitmapBlock],
    index_blocks: &mut [IBlock],
    inum: usize,
    count: usize
) {
    let inode = unsafe { get_inode(index_blocks, inum) }.clone();
    if inode.block_count < count as u32 {
        extend_data_block_of_inode(index_bitmap_blocks, data_bitmap_blocks, index_blocks, inum, count);
    } else if inode.block_count > count as u32 {
        shrink_data_block_of_inode(index_bitmap_blocks, data_bitmap_blocks, index_blocks, inum, count);
    }
}

/// 获取数据块索引，如果不够则扩充
pub fn get_or_extend_dnum<'a>(
    index_bitmap_blocks: &mut [BitmapBlock],
    data_bitmap_blocks: &mut [BitmapBlock],
    index_blocks: &'a mut [IBlock],
    inum: usize,
    index: usize
) -> &'a mut u32 {
    // 扩充
    extend_data_block_of_inode(index_bitmap_blocks, data_bitmap_blocks, index_blocks, inum, index + 1);
    get_dnum_mut(index_blocks, inum, index)
}


/// 将 pos 转化为 (dnum, offset)
pub fn transform_pos(index_blocks: &[IBlock], inum: usize, pos: usize) -> (usize, usize) {
    let inode = unsafe { get_inode(index_blocks, inum) };
    let block_index = pos / 4096;
    let offset = pos % 4096;

    if block_index >= inode.block_count as usize || block_index >= inode.size as usize {
        panic!("pos out of range");
    }

    let dnum = get_dnum(index_blocks, inum, block_index);
    (*dnum as usize, offset)
}

/// 获取数据块的大小
pub fn get_size_of_data_block(inode: &INode, index: usize) -> usize {
    if index >= inode.block_count as usize {
        panic!("index out of range");
    }

    if index == inode.block_count as usize - 1 {
        (inode.size as usize - 1) % 4096 + 1
    } else {
        4096
    }
}

/// 释放 inode 和它连接的数据块
pub fn free_inode(
    index_bitmap_blocks: &mut[BitmapBlock],
    block_bitmap_blocks: &mut[BitmapBlock],
    index_blocks: &mut [IBlock],
    inum: usize
) {
    let inode = unsafe { get_inode(index_blocks, inum) }.clone();

    for i in 0..inode.block_count {
        let dnum = get_dnum(index_blocks, inum, i as usize);
        set_state(block_bitmap_blocks, *dnum as usize, false);
    }

    if inode.block_indirect != 0 {
        set_state(index_bitmap_blocks, inode.block_indirect as usize, false);
    }

    let inode = unsafe { get_inode_mut(index_blocks, inum) };
    *inode = unsafe { std::mem::zeroed() };

    set_state(index_bitmap_blocks, inum, false);
}


/// 读取数据
pub fn read_data(
    data_blocks: &[DataBlock],
    index_blocks: &[IBlock],
    inum: usize,
    start_pos: usize,
    buf: &mut [u8]
) {
    let inode = unsafe { get_inode(index_blocks, inum) };
    let mut readed = 0;
    while readed < buf.len() {
        let (dnum, offset) = transform_pos(index_blocks, inum, start_pos + readed);

        let data = get_data_block(data_blocks, dnum);

        let block_index = (start_pos + readed) / 4096;
        let size_of_data = get_size_of_data_block(inode, block_index);

        let len = std::cmp::min(buf.len() - readed, size_of_data - offset);

        buf[readed..readed + len].copy_from_slice(&data[offset..offset + len]);
        readed += len;
    }
}

/// 写入数据
pub fn write_data(
    data_blocks: &mut [DataBlock],
    index_blocks: &mut [IBlock],
    inum: usize,
    start_pos: usize,
    buf: &[u8]
) {
    let inode = unsafe { get_inode(index_blocks, inum) };

    let mut written = 0;
    let mut cnt = 0;
    while written < buf.len() {
        cnt = cnt + 1;
        let (dnum, offset) = transform_pos(index_blocks, inum, start_pos + written);

        let data = get_data_block_mut(data_blocks, dnum);

        let block_index = (start_pos + written) / 4096;
        let size_of_data = get_size_of_data_block(inode, block_index);

        let len = std::cmp::min(buf.len() - written, size_of_data - offset);

        data[offset..offset + len].copy_from_slice(&buf[written..written + len]);
        written += len;
    }
}

/// 写入数据，自动调整大小
pub fn write_data_auto_resize(
    index_bitmap_blocks: &mut [BitmapBlock],
    data_bitmap_blocks: &mut [BitmapBlock],
    index_blocks: &mut [IBlock],
    data_blocks: &mut [DataBlock],
    inum: usize,
    start_pos: usize,
    buf: &[u8]
) {
    let new_size = start_pos + buf.len();
    let inode = unsafe { get_inode(index_blocks, inum) };
    if new_size != inode.size as usize {
        resize(index_bitmap_blocks, data_bitmap_blocks, index_blocks, inum, new_size);
    }

    write_data(data_blocks, index_blocks, inum, start_pos, buf);
}

/// 设置大小
pub fn resize(
    index_bitmap_blocks: &mut [BitmapBlock],
    data_bitmap_blocks: &mut [BitmapBlock],
    index_blocks: &mut [IBlock],
    inum: usize,
    new_size: usize
) {
    let new_block_count = (new_size + 4095) / 4096;

    resize_data_block_of_inode(index_bitmap_blocks, data_bitmap_blocks, index_blocks, inum, new_block_count);

    let inode = unsafe { get_inode_mut(index_blocks, inum) };
    inode.size = new_size as u32;
}

/// 读取数据结构
pub fn read_data_struct<T: for<'a> Deserialize<'a>>(
    data_blocks: &[DataBlock],
    index_blocks: &[IBlock],
    inum: usize,
    start_pos: usize
) -> T {
    let mut len_buf = vec![0u8; 4];
    read_data(data_blocks, index_blocks, inum, start_pos, &mut len_buf);
    let len = u32::from_le_bytes(len_buf.try_into().unwrap()) as usize;

    let mut buf = vec![0u8; len];
    read_data(data_blocks, index_blocks, inum, start_pos + 4, &mut buf);
    serde_json::from_slice(&buf).unwrap()
}

/// 写入数据结构
pub fn write_data_struct<T: Serialize>(
    data_blocks: &mut [DataBlock],
    index_blocks: &mut [IBlock],
    inum: usize,
    start_pos: usize,
    data: &T
) -> usize {
    let buf = serde_json::to_vec(data).unwrap();
    let len = buf.len() as u32;
    write_data(data_blocks, index_blocks, inum, start_pos, &len.to_le_bytes());
    write_data(data_blocks, index_blocks, inum, start_pos + 4, &buf);

    4 + buf.len()
}

/// 读取数据结构，自动调整大小
pub fn write_data_struct_auto_resize<T: Serialize>(
    index_bitmap_blocks: &mut [BitmapBlock],
    data_bitmap_blocks: &mut [BitmapBlock],
    index_blocks: &mut [IBlock],
    data_blocks: &mut [DataBlock],
    inum: usize,
    start_pos: usize,
    data: &T
) -> usize {
    let buf = serde_json::to_vec(data).unwrap();
    let len = buf.len() as u32;

    let new_size = start_pos + 4 + buf.len();
    let inode = unsafe { get_inode(index_blocks, inum) };
    if new_size != inode.size as usize {
        resize(index_bitmap_blocks, data_bitmap_blocks, index_blocks, inum, new_size);
    }

    write_data(data_blocks, index_blocks, inum, start_pos, &len.to_le_bytes());
    write_data(data_blocks, index_blocks, inum, start_pos + 4, &buf);

    4 + buf.len()
}

#[cfg(test)]
mod test {
    use std::mem::ManuallyDrop;

    use super::*;

    #[test]
    fn test_transform_pos() {
        let inode = INode {
            size: 3134333,
            is_dir: false,
            atime: 0,
            ctime: 0,
            mtime: 0,
            block_count: 15,
            block_direct: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            block_indirect: 1,
        };

        let mut idx = unsafe { std::mem::zeroed::<[u32; 1024]>() };
        idx[0] = 12;
        idx[1] = 13;
        idx[2] = 14;


        let mut blocks = vec![IBlock {
            inodes: ManuallyDrop::new([inode; 32]),
        }, IBlock {
            idx: ManuallyDrop::new(idx),
        }];

        assert_eq!(transform_pos(&blocks, 0, 0), (0, 0));
        assert_eq!(transform_pos(&blocks, 0, 4096), (1, 0));
        assert_eq!(transform_pos(&blocks, 0, 4096 * 12), (12, 0));
        assert_eq!(transform_pos(&blocks, 0, 4096 * 12 + 1), (12, 1));
        assert_eq!(transform_pos(&blocks, 0, 4096 * 12 + 4095), (12, 4095));
        assert_eq!(transform_pos(&blocks, 0, 4096 * 13), (13, 0));
        assert_eq!(transform_pos(&blocks, 0, 4096 * 13 + 1), (13, 1));
        assert_eq!(transform_pos(&blocks, 0, 4096 * 13 + 4095), (13, 4095));

        let old_idx = idx.clone();
        let old_blocks = vec![IBlock {
            idx: ManuallyDrop::new(old_idx),
        }];


        let dnum = get_dnum_mut(&mut blocks, 0, 12);
        *dnum = 100;

        assert_eq!(transform_pos(&blocks, 0, 4096 * 12), (100, 0));
        assert_eq!(transform_pos(&blocks, 0, 4096 * 12 + 1), (100, 1));
        assert_eq!(transform_pos(&blocks, 0, 4096 * 12 + 4095), (100, 4095));

        unsafe { assert_ne!(old_blocks[0].idx, blocks[0].idx); }
    }


    #[test]
    fn test_read_write_data() {
        let mut disk = Disk::new();
        disk.i_bitmaps[0].bitmaps[0] = 3;       // ..00011
        disk.d_bitmaps[0].bitmaps[0] = 16383;   // 14 个 1

        let inode = unsafe { get_inode_mut(&mut disk.i_blocks, 0) };
        *inode = INode {
            size: 23423,
            is_dir: false,
            atime: 0,
            ctime: 0,
            mtime: 0,
            block_count: 14,
            block_direct: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            block_indirect: 1,
        };

        let indirect_1 = unsafe { get_indirect_dnum_mut(&mut disk.i_blocks, 1, 0) };
        *indirect_1 = 12;

        let indirect_2 = unsafe { get_indirect_dnum_mut(&mut disk.i_blocks, 1, 1) };
        *indirect_2 = 13;

        let mut buf = vec![0; 4096 * 3 + 234];
        for i in 0..buf.len() {
            buf[i] = i as u8;
        }

        let data_blocks = &mut disk.d_blocks;
        let blocks = &mut disk.i_blocks;

        write_data(data_blocks, blocks, 0, 0, &buf);

        let mut read_buf = vec![0; 4096 * 3 + 234];
        read_data(data_blocks, blocks, 0, 0, &mut read_buf);

        assert_eq!(buf, read_buf);


        let mut buf = vec![0; 32];
        for i in 0..buf.len() {
            buf[i] = i as u8;
        }

        write_data(data_blocks, blocks, 0, 4096 * 5 + 23, &buf);

        let mut read_buf = vec![0; 32];
        read_data(data_blocks, blocks, 0, 4096 * 5 + 23, &mut read_buf);

        assert_eq!(buf, read_buf);
    }

    #[test]
    fn test_get_data_size() {
        let inode = INode {
            size: 4096 * 3 + 234,
            is_dir: false,
            atime: 0,
            ctime: 0,
            mtime: 0,
            block_count: 4,
            block_direct: [0, 1, 2, 3, 0, 0, 0, 0, 0, 0, 0, 0],
            block_indirect: 0,
        };

        assert_eq!(get_size_of_data_block(&inode, 0), 4096);
        assert_eq!(get_size_of_data_block(&inode, 1), 4096);
        assert_eq!(get_size_of_data_block(&inode, 2), 4096);
        assert_eq!(get_size_of_data_block(&inode, 3), 234);
    }

    #[should_panic]
    #[test]
    fn test_get_dnum_panic() {
        let inode = INode {
            size: 4096 * 3 + 234,
            is_dir: false,
            atime: 0,
            ctime: 0,
            mtime: 0,
            block_count: 4,
            block_direct: [0, 1, 2, 3, 0, 0, 0, 0, 0, 0, 0, 0],
            block_indirect: 0,
        };

        get_size_of_data_block(&inode, 5);
    }

    #[test]
    fn test_resize() {
        let mut disk = Disk::new();

        unsafe {
            let inode = get_inode_mut(&mut disk.i_blocks, 0);

            *inode = INode {
                size: 4096 * 3 + 234,
                is_dir: false,
                atime: 0,
                ctime: 0,
                mtime: 0,
                block_count: 4,
                block_direct: [0, 1, 2, 3, 0, 0, 0, 0, 0, 0, 0, 0],
                block_indirect: 0,
            };
            disk.d_bitmaps[0].bitmaps[0] = 15;  // ...001111
            disk.i_bitmaps[0].bitmaps[0] = 1;   // ...000001
        }

        let i_blocks = &mut disk.i_blocks;
        let i_bitmaps = &mut disk.i_bitmaps;
        let d_bitmaps = &mut disk.d_bitmaps;

        resize(i_bitmaps, d_bitmaps, i_blocks, 0, 4096 * 3 + 234 + 4096);

        let inode = unsafe { get_inode(i_blocks, 0) };
        assert_eq!(inode.size, 4096 * 3 + 234 + 4096);
        assert_eq!(inode.block_count, 5);

        assert_eq!(get_state(d_bitmaps, 0), true);
        assert_eq!(get_state(d_bitmaps, 1), true);
        assert_eq!(get_state(d_bitmaps, 2), true);
        assert_eq!(get_state(d_bitmaps, 3), true);
        assert_eq!(get_state(d_bitmaps, 4), true);

        assert_eq!(get_state(d_bitmaps, 5), false);
        assert_eq!(get_state(d_bitmaps, 6), false);

        resize(i_bitmaps, d_bitmaps, i_blocks, 0, 4096 * 3 + 234);

        let inode = unsafe { get_inode_mut(i_blocks, 0) };
        assert_eq!(inode.size, 4096 * 3 + 234);
        assert_eq!(inode.block_count, 4);

        assert_eq!(get_state(d_bitmaps, 0), true);
        assert_eq!(get_state(d_bitmaps, 1), true);
        assert_eq!(get_state(d_bitmaps, 2), true);
        assert_eq!(get_state(d_bitmaps, 3), true);

        assert_eq!(get_state(d_bitmaps, 4), false);
        assert_eq!(get_state(d_bitmaps, 5), false);
        assert_eq!(get_state(d_bitmaps, 6), false);

        resize(i_bitmaps, d_bitmaps, i_blocks, 0, 0);
        let inode = unsafe { get_inode_mut(i_blocks, 0) };
        assert_eq!(inode.size, 0);
        assert_eq!(inode.block_count, 0);

        assert_eq!(get_state(d_bitmaps, 0), false);
        assert_eq!(get_state(d_bitmaps, 1), false);
        assert_eq!(get_state(d_bitmaps, 2), false);
        assert_eq!(get_state(d_bitmaps, 3), false);
    }

    #[test]
    fn test_serde() {
        let data = DirectoryData {
            entries: vec![
                DirectoryEntry {
                    name: "test".to_string(),
                    inum: 1,
                },
                DirectoryEntry {
                    name: "test2".to_string(),
                    inum: 2,
                },
            ],
        };

        let buf = serde_json::to_vec(&data).unwrap();
        let read_data: DirectoryData = serde_json::from_slice(&buf).unwrap();

        assert_eq!(data, read_data);
    }

    #[test]
    fn test_rw_data_struct() {
        let mut disk = Disk::new();
        let inode = unsafe { get_inode_mut(&mut disk.i_blocks, 0) };

        *inode = INode {
            size: 4096 * 3 + 234,
            is_dir: false,
            atime: 0,
            ctime: 0,
            mtime: 0,
            block_count: 4,
            block_direct: [0, 1, 2, 3, 0, 0, 0, 0, 0, 0, 0, 0],
            block_indirect: 0,
        };
        disk.d_bitmaps[0].bitmaps[0] = 15;  // ...001111
        disk.i_bitmaps[0].bitmaps[0] = 1;   // ...000001

        let disk_ptr = &mut disk as *mut Box<Disk>;

        let i_blocks = &mut disk.i_blocks;

        let i_blocks_ptr = i_blocks as *mut [IBlock; INDEX_BLOCK_COUNT];

        let inode = unsafe { get_inode_mut(i_blocks, 0) };

        let data = DirectoryData {
            entries: vec![
                DirectoryEntry {
                    name: "test".to_string(),
                    inum: 1,
                },
                DirectoryEntry {
                    name: "test2".to_string(),
                    inum: 2,
                },
            ],
        };

        unsafe {
            write_data_struct(disk.d_blocks.as_mut(), i_blocks_ptr.as_mut().unwrap(), 0, 0, &data);
        }

        let read_data = read_data_struct(disk.d_blocks.as_ref(), unsafe { i_blocks_ptr.as_mut() }.unwrap(), 0, 0);
        assert_eq!(data, read_data);

        unsafe {
            let disk = &mut *disk_ptr;
            println!("before size: {:?}", inode);
            write_data_struct_auto_resize(
                disk.i_bitmaps.as_mut(),
                disk.d_bitmaps.as_mut(),
                disk.i_blocks.as_mut(),
                disk.d_blocks.as_mut(),
                0, 0, &data
            );
            println!("after size: {:?}", inode);
        }

        let read_data = read_data_struct(disk.d_blocks.as_ref(), unsafe { i_blocks_ptr.as_mut() }.unwrap(), 0, 0);
        assert_eq!(data, read_data);
    }

    #[test]
    fn test_rw() {
        let mut disk = Disk::new();
        let inode = unsafe { get_inode_mut(&mut disk.i_blocks, 0) };

        *inode = INode {
            size: 0,
            is_dir: false,
            atime: 0,
            ctime: 0,
            mtime: 0,
            block_count: 0,
            block_direct: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            block_indirect: 0,
        };
        disk.d_bitmaps[0].bitmaps[0] = 0;
        disk.i_bitmaps[0].bitmaps[0] = 1;

        let i_bitmaps = &mut disk.i_bitmaps;
        let d_bitmaps = &mut disk.d_bitmaps;
        let i_blocks = &mut disk.i_blocks;
        let d_blocks = &mut disk.d_blocks;

        let mut buf = vec![0; 4096 * 3 + 234];
        for i in 0..buf.len() {
            buf[i] = i as u8;
        }

        write_data_auto_resize(
            i_bitmaps,
            d_bitmaps,
            i_blocks,
            d_blocks,
            0, 0, &buf
        );

        assert_eq!(unsafe { get_inode(i_blocks, 0) }.block_count, 4);
        assert_eq!(unsafe { get_inode(i_blocks, 0) }.size, 4096 * 3 + 234);

        let mut read_buf = vec![0; 4096 * 3 + 234];
        read_data(d_blocks, i_blocks, 0, 0, &mut read_buf);

        assert_eq!(buf, read_buf);
    }

    #[test]
    fn test_extend_shrink_data_block() {
        let mut disk = Disk::new();
        let inode = unsafe { get_inode_mut(&mut disk.i_blocks, 0) };
        *inode = INode {
            size: 0,
            is_dir: false,
            atime: 0,
            ctime: 0,
            mtime: 0,
            block_count: 0,
            block_direct: [0; 12],
            block_indirect: 0,
        };

        disk.i_bitmaps[0].bitmaps[0] = 1;

        let i_bitmaps = &mut disk.i_bitmaps;
        let d_bitmaps = &mut disk.d_bitmaps;
        let i_blocks = &mut disk.i_blocks;
        let d_blocks = &mut disk.d_blocks;

        extend_data_block_of_inode(
            i_bitmaps,
            d_bitmaps,
            i_blocks,
            0, 3,
        );

        let inode = unsafe { get_inode(i_blocks, 0) };

        assert_eq!(inode.block_count, 3);
        assert_eq!(inode.block_direct[0], 0);
        assert_eq!(inode.block_direct[1], 1);
        assert_eq!(inode.block_direct[2], 2);

        assert_eq!(inode.block_direct[3], 0);
        assert_eq!(inode.block_direct[4], 0);

        assert_eq!(get_state(d_bitmaps, 0), true);
        assert_eq!(get_state(d_bitmaps, 1), true);
        assert_eq!(get_state(d_bitmaps, 2), true);

        assert_eq!(get_state(d_bitmaps, 3), false);
        assert_eq!(get_state(d_bitmaps, 4), false);
        assert_eq!(get_state(d_bitmaps, 5), false);

        println!("{:?}", inode);


        extend_data_block_of_inode(
            i_bitmaps,
            d_bitmaps,
            i_blocks,
            0, 20,
        );

        let inode = unsafe { get_inode(i_blocks, 0) };

        println!("{:?}", inode);

        assert_eq!(inode.block_count, 20);
        assert_eq!(inode.block_direct[4], 4);
        assert_eq!(inode.block_direct[5], 5);
        assert_eq!(inode.block_direct[6], 6);
        assert_eq!(inode.block_direct[8], 8);
        assert_eq!(inode.block_direct[11], 11);

        assert_ne!(inode.block_indirect, 0);

        unsafe {
            for i in 12u32..18u32 {
                assert_eq!(*get_indirect_dnum(i_blocks, inode.block_indirect as usize, i as usize - 12), i);
            }

            assert_eq!(*get_indirect_dnum(i_blocks, inode.block_indirect as usize, 100), 0);
            println!("{:?}", get_indirect_block(i_blocks, inode.block_indirect as usize));
        }


        shrink_data_block_of_inode(
            i_bitmaps,
            d_bitmaps,
            i_blocks,
            0, 10,
        );

        let inode = unsafe { get_inode(i_blocks, 0) };

        println!("{:?}", inode);

        assert_eq!(inode.block_count, 10);
        assert_eq!(inode.block_direct[4], 4);
        assert_eq!(inode.block_direct[5], 5);
        assert_eq!(inode.block_direct[6], 6);
        assert_eq!(inode.block_direct[8], 8);

        assert_eq!(inode.block_direct[11], 0);
        assert_eq!(inode.block_indirect, 0);


        shrink_data_block_of_inode(
            i_bitmaps,
            d_bitmaps,
            i_blocks,
            0, 0,
        );

        let inode = unsafe { get_inode(i_blocks, 0) };

        println!("{:?}", inode);

        assert_eq!(inode.block_count, 0);
        assert_eq!(inode.block_direct[4], 0);
        assert_eq!(inode.block_direct[5], 0);
        assert_eq!(inode.block_direct[6], 0);
        assert_eq!(inode.block_direct[8], 0);
    }

    #[test]
    fn test_free_inode() {
        let mut disk = Disk::new();
        let inode = unsafe { get_inode_mut(&mut disk.i_blocks, 0) };
        *inode = INode {
            size: 0,
            is_dir: false,
            atime: 0,
            ctime: 0,
            mtime: 0,
            block_count: 0,
            block_direct: [0; 12],
            block_indirect: 0,
        };

        disk.i_bitmaps[0].bitmaps[0] = 1;

        let i_bitmaps = &mut disk.i_bitmaps;
        let d_bitmaps = &mut disk.d_bitmaps;
        let i_blocks = &mut disk.i_blocks;
        let d_blocks = &mut disk.d_blocks;

        extend_data_block_of_inode(
            i_bitmaps,
            d_bitmaps,
            i_blocks,
            0, 20,
        );

        let inode = unsafe { get_inode(i_blocks, 0) };
        assert_eq!(inode.block_count, 20);

        free_inode(i_bitmaps, d_bitmaps, i_blocks, 0);

        for i in 0..20 {
            assert_eq!(get_state(d_bitmaps, i as usize), false);
        }

        assert_eq!(i_bitmaps[0].bitmaps[0], 0);
        assert_eq!(d_bitmaps[0].bitmaps[0], 0);
    }
}
