use std::collections::HashMap;

#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub enum AccessMode {
    Read,
    Write,
    ReadWrite,
}

#[derive(Hash, Eq, PartialEq)]
struct RWTableEntry {
    pid: usize,                         // 进程 ID
    path: String,                       // 文件路径
    mode: AccessMode,                   // 文件打开模式
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
        let entry = RWTableEntry{
            pid,
            path: path.to_string(),
            mode,
        };
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
    pub fn open(&mut self, pid: usize, path: &str, mode: AccessMode) -> usize {
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
                self.file_rw_table.set_read(&entry.path, entry.mode == AccessMode::Read || entry.mode == AccessMode::ReadWrite);
                self.file_rw_table.set_write(&entry.path, entry.mode == AccessMode::Write || entry.mode == AccessMode::ReadWrite);

            });
    }

    /// 关闭文件
    pub fn close(&mut self, id: usize) {
        self.open_table.close(id);
        self.update_state();
    }

    /// 是否可以写文件
    pub fn can_write(&self, path: &str) -> bool {
        self.file_rw_table.can_write(path)
    }

    /// 是否文件已经打开
    pub fn is_open(&self, pid: usize, path: &str, mode: AccessMode) -> bool {
        self.open_table.entries.iter()
            .any(|(entry, deleted)| {
                if *deleted {
                    return false;
                }
                entry.pid == pid && entry.path == path && entry.mode == mode
            })
    }

    /// 是否文件已经删除
    fn is_deleted(&self, id: usize) -> bool {
        self.open_table.entries[id].1
    }

    /// 是否仍然打开
    pub fn already_open(&self, id: usize) -> bool {
        self.is_deleted(id)
    }

    /// 根据 id 获得读写模式
    pub fn access_mode(&self, id: usize) -> Option<AccessMode> {
        if self.is_deleted(id) {
            None
        } else {
            Some(self.open_table.entries[id].0.mode)
        }
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_rw_manager() {
        let mut rw_manager = RWManager::new();
        let x = rw_manager.open(1, "test.txt", AccessMode::Read);
        assert_eq!(rw_manager.can_write("test.txt"), true);

        rw_manager.close(x);

        let x = rw_manager.open(1, "test.txt", AccessMode::Write);
        assert_eq!(rw_manager.can_write("test.txt"), false);

        rw_manager.close(x);

        let x = rw_manager.open(1, "test.txt", AccessMode::ReadWrite);
        assert_eq!(rw_manager.can_write("test.txt"), false);
        assert_eq!(rw_manager.is_open(1, "test.txt", AccessMode::Read), false);
        assert_eq!(rw_manager.is_open(1, "test.txt", AccessMode::Write), false);
        assert_eq!(rw_manager.is_open(1, "test.txt", AccessMode::ReadWrite), true);

        assert_eq!(rw_manager.can_write("test.txt"), false);
        rw_manager.close(x);

        assert_eq!(rw_manager.can_write("test.txt"), true);
        assert_eq!(rw_manager.is_open(1, "test.txt", AccessMode::Read), false);
    }
}