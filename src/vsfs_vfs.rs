use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

use crate::path::Path;
use crate::repr::Disk;
use crate::rw::{AccessMode, RWManager};
use crate::vfs::{VirtualFile, VirtualFileDescription, VirtualFileSystem};
use crate::vsfs;

pub struct VerySimpleFile {
    path: Path,
    mode: AccessMode,
    position: usize,
    id: usize,
}

impl VirtualFile for VerySimpleFile {
    fn path(&self) -> &Path {
        &self.path
    }

    fn mode(&self) -> AccessMode {
        self.mode
    }

    fn position(&self) -> usize {
        self.position
    }

    fn set_position(&mut self, pos: usize) {
        self.position = pos;
    }
}






pub struct VerySimpleFileDescription {

}

impl VirtualFileDescription for VerySimpleFileDescription {
    fn is_dir(&self) -> bool {
        todo!()
    }

    fn name(&self) -> &str {
        todo!()
    }

    fn ctime(&self) -> u64 {
        todo!()
    }

    fn mtime(&self) -> u64 {
        todo!()
    }

    fn size(&self) -> u64 {
        todo!()
    }
}






#[derive(Debug)]
pub enum VerySimpleError {
    FileCannotWrite,
    FileNotOpen,
    FileNotExist,
}

impl Display for VerySimpleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            VerySimpleError::FileCannotWrite => write!(f, "File cannot write"),
            VerySimpleError::FileNotOpen => write!(f, "File not open"),
            VerySimpleError::FileNotExist => write!(f, "File not exist"),
        }
    }
}

impl Error for VerySimpleError {

}







pub struct VerySimpleFileSystem<'disk> {
    rw: RWManager,
    disk: &'disk mut Disk
}


impl<'disk> VirtualFileSystem for VerySimpleFileSystem<'disk> {
    type File = VerySimpleFile;
    type Error = VerySimpleError;
    type FileDescription = VerySimpleFileDescription;

    fn init(&mut self) -> Result<(), Self::Error> {
        vsfs::init(self.disk);
        Ok(())
    }

    fn open(&mut self, path: &Path, mode: AccessMode) -> Result<Self::File, Self::Error> {

        // 检查是否可以打开
        match mode {
            AccessMode::Read => {}
            AccessMode::Write | AccessMode::ReadWrite => {
                if !self.rw.file_can_write(&path.to_str()) {
                    return Err(VerySimpleError::FileCannotWrite)
                }
            }
        }

        if !vsfs::exists(self.disk, path) {
            return Err(VerySimpleError::FileNotExist)
        }

        // 打开文件
        let id = self.rw.open_file(0, &path.to_str(), mode);

        // 返回文件
        Ok(VerySimpleFile {
            path: path.clone(),
            mode,
            position: 0,
            id,
        })
    }

    fn close(&mut self, file: Self::File) -> Result<(), Self::Error> {

        if self.rw.if_delete(file.id) {
            return Err(VerySimpleError::FileNotOpen)
        }

        self.rw.close_file(file.id);
        Ok(())
    }

    fn read(&self, file: &mut Self::File, buf: &mut [u8]) -> Result<usize, Self::Error> {

        // vsfs::read_file(self.disk, &file.path, file.position, buf)?;
        todo!()
    }

    fn write(&mut self, file: &mut Self::File, buf: &[u8]) -> Result<usize, Self::Error> {
        todo!()
    }

    fn list(&self, path: &Path) -> Result<Vec<Self::FileDescription>, Self::Error> {
        todo!()
    }

    fn mkdir(&mut self, path: &Path) -> Result<(), Self::Error> {
        todo!()
    }

    fn rmdir(&mut self, path: &Path) -> Result<(), Self::Error> {
        todo!()
    }

    fn exists(&self, path: &Path) -> Result<bool, Self::Error> {
        todo!()
    }
}


impl<'disk> VerySimpleFileSystem<'disk> {
    pub fn new(disk: &'disk mut Disk) -> Self {
        VerySimpleFileSystem {
            rw: RWManager::new(),
            disk,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[should_panic]
    #[test]
    fn test_vfs_open_panic() {
        let mut disk = Disk::new();
        let mut fs = VerySimpleFileSystem::new(&mut disk);

        let path = Path::from_str("/test.txt").unwrap();
        let file = fs.open(&path, AccessMode::ReadWrite).unwrap();
    }
}