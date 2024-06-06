use std::collections::HashMap;

#[derive(Hash, Eq, PartialEq, Copy, Clone)]
pub enum AccessMode {
    Read,
    Write,
    ReadWrite,
}

#[derive(Hash, Eq, PartialEq)]
struct OpenFileEntry {
    pid: usize,                         // 进程 ID
    path: String,                       // 文件路径
    mode: AccessMode,                   // 文件打开模式
}

#[derive(Hash, Eq, PartialEq)]
struct OpenDirectoryEntry {
    pid: usize,                         // 进程 ID
    path: String,                       // 目录路径
}

#[derive(Hash, Eq, PartialEq)]
enum RWTableEntry {
    File(OpenFileEntry),
    Directory(OpenDirectoryEntry),
}

struct OpenTable {
    entries: Vec<(RWTableEntry, bool)>,         // 打开文件表，第二个元素表示是否被删除
}


impl OpenTable {
    fn new() -> Self {
        OpenTable {
            entries: Vec::new(),
        }
    }

    /// 打开文件
    fn open_file(&mut self, pid: usize, path: &str, mode: AccessMode) -> usize {
        let entry = RWTableEntry::File(OpenFileEntry {
            pid,
            path: path.to_string(),
            mode,
        });
        self.entries.push((entry, false));
        self.entries.len() - 1
    }

    /// 打开目录
    fn open_directory(&mut self, pid: usize, path: &str) -> usize {
        let entry = RWTableEntry::Directory(OpenDirectoryEntry {
            pid,
            path: path.to_string(),
        });
        self.entries.push((entry, false));
        self.entries.len() - 1
    }

    /// 关闭
    fn close(&mut self, id: usize) {
        self.entries[id].1 = true;
    }
}


struct FileRWTable {
    map: HashMap<String, u8>,           // 文件读写状态表
}


impl FileRWTable {
    fn new() -> Self {
        FileRWTable {
            map: HashMap::new(),
        }
    }

    /// 是否可以写文件
    fn can_write(&self, path: &str) -> bool {
        match self.map.get(path) {
            Some(state) => state & 0b00000010u8 == 0,
            None => true,
        }
    }

    /// 设置文件读状态
    fn set_read(&mut self, path: &str, read: bool) {
        let state = self.map.entry(path.to_string())
            .or_insert(0b00000000);
        *state |= read as u8;
    }

    /// 设置文件写状态
    fn set_write(&mut self, path: &str, write: bool) {
        let state = self.map.entry(path.to_string())
            .or_insert(0b00000000);
        *state |= (write as u8) << 1;
    }
}


pub struct RWManager {
    open_table: OpenTable,              // 打开文件表
    file_rw_table: FileRWTable,         // 文件读写状态表
}


impl RWManager {
    pub fn new() -> Self {
        RWManager {
            open_table: OpenTable::new(),
            file_rw_table: FileRWTable::new(),
        }
    }

    /// 打开文件
    pub fn open_file(&mut self, pid: usize, path: &str, mode: AccessMode) -> usize {
        let entry_id = self.open_table.open_file(pid, path, mode);
        match mode {
            AccessMode::Read => {
                self.file_rw_table.set_read(path, true);
            }
            AccessMode::Write => {
                self.file_rw_table.set_write(path, true);
            }
            AccessMode::ReadWrite => {
                self.file_rw_table.set_read(path, true);
                self.file_rw_table.set_write(path, true);
            }
        }
        entry_id
    }

    /// 更新状态
    fn update_state(&mut self) {
        self.file_rw_table.map.clear();
        self.open_table.entries
            .iter()
            .for_each(|(entry, deleted)| {
                if *deleted {
                    return;
                }
                match entry {
                    RWTableEntry::File(file) => {
                        self.file_rw_table.set_read(&file.path, file.mode == AccessMode::Read || file.mode == AccessMode::ReadWrite);
                        self.file_rw_table.set_write(&file.path, file.mode == AccessMode::Write || file.mode == AccessMode::ReadWrite);
                    }
                    RWTableEntry::Directory(_) => {}
                }
            });
    }

    /// 打开目录
    pub fn open_directory(&mut self, pid: usize, path: &str) -> usize {
        self.open_table.open_directory(pid, path)
    }

    /// 关闭文件
    pub fn close_file(&mut self, id: usize) {
        self.open_table.close(id);
        self.update_state();
    }

    /// 关闭目录
    pub fn close_directory(&mut self, id: usize) {
        self.open_table.close(id);
    }

    /// 是否可以写文件
    pub fn file_can_write(&self, path: &str) -> bool {
        self.file_rw_table.can_write(path)
    }

    /// 是否目录已经打开
    pub fn dir_is_open(&self, pid: usize, path: &str) -> bool {
        self.open_table.entries.iter()
            .any(|(entry, deleted)| {
                if *deleted {
                    return false;
                }
                match entry {
                    RWTableEntry::File(_) => false,
                    RWTableEntry::Directory(dir) => dir.pid == pid && dir.path == path,
                }
            })
    }

    /// 是否文件已经打开
    pub fn file_is_open(&self, pid: usize, path: &str, mode: AccessMode) -> bool {
        self.open_table.entries.iter()
            .any(|(entry, deleted)| {
                if *deleted {
                    return false;
                }
                match entry {
                    RWTableEntry::File(file) => file.pid == pid && file.path == path && file.mode == mode,
                    RWTableEntry::Directory(_) => false,
                }
            })
    }

    /// 是否文件已经删除
    pub fn if_delete(&self, id: usize) -> bool {
        self.open_table.entries[id].1
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_rw_manager() {
        let mut rw_manager = RWManager::new();
        let x = rw_manager.open_file(1, "test.txt", AccessMode::Read);
        assert_eq!(rw_manager.file_can_write("test.txt"), true);

        rw_manager.close_file(x);

        let x = rw_manager.open_file(1, "test.txt", AccessMode::Write);
        assert_eq!(rw_manager.file_can_write("test.txt"), false);

        rw_manager.close_file(x);

        let x = rw_manager.open_file(1, "test.txt", AccessMode::ReadWrite);
        assert_eq!(rw_manager.file_can_write("test.txt"), false);
        assert_eq!(rw_manager.file_is_open(1, "test.txt", AccessMode::Read), false);
        assert_eq!(rw_manager.file_is_open(1, "test.txt", AccessMode::Write), false);
        assert_eq!(rw_manager.file_is_open(1, "test.txt", AccessMode::ReadWrite), true);

        assert_eq!(rw_manager.file_can_write("test.txt"), false);
        rw_manager.close_file(x);

        assert_eq!(rw_manager.file_can_write("test.txt"), true);
        assert_eq!(rw_manager.file_is_open(1, "test.txt", AccessMode::Read), false);
    }

    #[test]
    fn test_rw_manager_dir() {
        let mut rw_manager = RWManager::new();

        let x = rw_manager.open_directory(1, "test");
        assert_eq!(rw_manager.dir_is_open(1, "test"), true);

        rw_manager.close_directory(x);
        assert_eq!(rw_manager.dir_is_open(1, "test"), false);
    }
}