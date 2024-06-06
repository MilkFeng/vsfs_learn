use std::collections::{HashMap, HashSet};

#[derive(Hash, Eq, PartialEq, Copy, Clone)]
pub enum AccessMode {
    Read,
    Write,
    ReadWrite,
}

#[derive(Hash, Eq, PartialEq)]
struct OpenFileEntry {
    pid: u32,                           // 进程 ID
    path: String,                       // 文件路径
    mode: AccessMode,                   // 文件打开模式
}

#[derive(Hash, Eq, PartialEq)]
struct OpenDirectoryEntry {
    pid: u32,                           // 进程 ID
    path: String,                       // 目录路径
}

#[derive(Hash, Eq, PartialEq)]
enum RWTableEntry {
    File(OpenFileEntry),
    Directory(OpenDirectoryEntry),
}

struct OpenTable {
    entries: HashSet<RWTableEntry>,     // 打开文件表
}


impl OpenTable {
    fn new() -> Self {
        OpenTable {
            entries: HashSet::new(),
        }
    }

    /// 打开文件
    fn open_file(&mut self, pid: u32, path: &str, mode: AccessMode) {
        self.entries.insert(RWTableEntry::File(OpenFileEntry {
            pid,
            path: path.to_string(),
            mode,
        }));
    }

    /// 打开目录
    fn open_directory(&mut self, pid: u32, path: &str) {
        self.entries.insert(RWTableEntry::Directory(OpenDirectoryEntry {
            pid,
            path: path.to_string(),
        }));
    }

    /// 关闭文件
    fn close_file(&mut self, pid: u32, path: &str) {
        self.entries.remove(&RWTableEntry::File(OpenFileEntry {
            pid,
            path: path.to_string(),
            mode: AccessMode::Read,
        }));
        self.entries.remove(&RWTableEntry::File(OpenFileEntry {
            pid,
            path: path.to_string(),
            mode: AccessMode::Write,
        }));
        self.entries.remove(&RWTableEntry::File(OpenFileEntry {
            pid,
            path: path.to_string(),
            mode: AccessMode::ReadWrite,
        }));
    }

    /// 关闭目录
    fn close_directory(&mut self, pid: u32, path: &str) {
        self.entries.remove(&RWTableEntry::Directory(OpenDirectoryEntry {
            pid,
            path: path.to_string(),
        }));
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
    fn new() -> Self {
        RWManager {
            open_table: OpenTable::new(),
            file_rw_table: FileRWTable::new(),
        }
    }

    /// 打开文件
    pub fn open_file(&mut self, pid: u32, path: &str, mode: AccessMode) {
        self.open_table.open_file(pid, path, mode);
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
    }

    /// 更新状态
    fn update_state(&mut self) {
        self.file_rw_table.map.clear();
        self.open_table.entries
            .iter()
            .for_each(|entry| {
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
    pub fn open_directory(&mut self, pid: u32, path: &str) {
        self.open_table.open_directory(pid, path);
    }

    /// 关闭文件
    pub fn close_file(&mut self, pid: u32, path: &str) {
        self.open_table.close_file(pid, path);
        self.update_state();
    }

    /// 关闭目录
    pub fn close_directory(&mut self, pid: u32, path: &str) {
        self.open_table.close_directory(pid, path);
    }

    /// 是否可以写文件
    pub fn file_can_write(&self, path: &str) -> bool {
        self.file_rw_table.can_write(path)
    }

    /// 是否目录已经打开
    pub fn dir_is_open(&self, pid: u32, path: &str) -> bool {
        self.open_table.entries.contains(&RWTableEntry::Directory(OpenDirectoryEntry {
            pid,
            path: path.to_string(),
        }))
    }

    /// 是否文件已经打开
    pub fn file_is_open(&self, pid: u32, path: &str, mode: AccessMode) -> bool {
        self.open_table.entries.contains(&RWTableEntry::File(OpenFileEntry {
            pid,
            path: path.to_string(),
            mode,
        }))
    }
}



#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_rw_manager() {
        let mut rw_manager = RWManager::new();
        rw_manager.open_file(1, "test.txt", AccessMode::Read);
        assert_eq!(rw_manager.file_can_write("test.txt"), true);
        rw_manager.open_file(1, "test.txt", AccessMode::Write);
        assert_eq!(rw_manager.file_can_write("test.txt"), false);
        rw_manager.open_file(1, "test.txt", AccessMode::ReadWrite);

        assert_eq!(rw_manager.file_is_open(1, "test.txt", AccessMode::Read), true);

        assert_eq!(rw_manager.file_can_write("test.txt"), false);
        rw_manager.close_file(1, "test.txt");

        assert_eq!(rw_manager.file_can_write("test.txt"), true);
        assert_eq!(rw_manager.file_is_open(1, "test.txt", AccessMode::Read), false);
    }

    #[test]
    fn test_rw_manager_dir() {
        let mut rw_manager = RWManager::new();

        rw_manager.open_directory(1, "test");
        assert_eq!(rw_manager.dir_is_open(1, "test"), true);

        rw_manager.close_directory(1, "test");
        assert_eq!(rw_manager.dir_is_open(1, "test"), false);
    }
}