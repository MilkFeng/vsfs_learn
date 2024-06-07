use std::cmp::min;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

use crate::path::Path;
use crate::repr::{Disk, INode};
use crate::rw::{AccessMode, RWManager};
use crate::rw::AccessMode::Read;
use crate::vfs::{VirtualFile, VirtualFileDescription, VirtualFileSystem};
use crate::vsfs;
use crate::vsfs::{update_access_time, update_modify_time};

#[derive(Debug)]
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

#[derive(Debug)]
pub struct VerySimpleFileDescription {
    inode: INode,
    name: String,
}

impl VirtualFileDescription for VerySimpleFileDescription {
    fn is_dir(&self) -> bool {
        self.inode.is_dir
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn ctime(&self) -> u64 {
        self.inode.ctime as u64
    }

    fn mtime(&self) -> u64 {
        self.inode.mtime as u64
    }

    fn size(&self) -> usize {
        self.inode.size as usize
    }
}


#[derive(Debug)]
pub enum VerySimpleError {
    UnknownError,
    FileCannotWrite,
    FileNotOpen,
    FileNotExist,
    InvalidPath,
    AccessError,
    VSFSError(vsfs::Error)
}

impl Display for VerySimpleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            VerySimpleError::FileCannotWrite => write!(f, "File cannot write"),
            VerySimpleError::FileNotOpen => write!(f, "File not open"),
            VerySimpleError::FileNotExist => write!(f, "File not exist"),
            VerySimpleError::UnknownError => write!(f, "unknown error"),
            VerySimpleError::VSFSError(error) => Display::fmt(error, f),
            VerySimpleError::InvalidPath => write!(f, "invalid path"),
            VerySimpleError::AccessError => write!(f, "access error. r, w, or rw"),
        }
    }
}

impl Error for VerySimpleError {}


pub struct VerySimpleFileSystem<'disk> {
    rw: RWManager,
    disk: &'disk mut Disk,
}


impl<'disk> VirtualFileSystem for VerySimpleFileSystem<'disk> {
    type File = VerySimpleFile;
    type Error = VerySimpleError;
    type FileDescription = VerySimpleFileDescription;

    fn init(&mut self) -> Result<(), Self::Error> {
        vsfs::init(&mut self.disk);
        Ok(())
    }

    fn create_file(&mut self, path: &Path) -> Result<Self::FileDescription, Self::Error> {
        let name = path.current()
            .ok_or(VerySimpleError::InvalidPath)?;
        let parent = path.clone().parent()
            .ok_or(VerySimpleError::InvalidPath)?;
        vsfs::create_file(&mut self.disk, &parent, name)
            .map_err(|err| VerySimpleError::VSFSError(err))?;

        let inode = vsfs::get_inode_by_path(&mut self.disk, path)
            .ok_or(VerySimpleError::UnknownError)?;

        Ok(VerySimpleFileDescription {
            inode: inode.clone(),
            name: name.clone(),
        })
    }

    fn delete_file(&mut self, path: &Path) -> Result<(), Self::Error> {
        vsfs::delete_file(&mut self.disk, &path)
            .map_err(|err| VerySimpleError::VSFSError(err))
    }


    fn open(&mut self, path: &Path, mode: AccessMode) -> Result<Self::File, Self::Error> {
        // 检查是否可以打开
        match mode {
            AccessMode::Read => {}
            AccessMode::Write | AccessMode::ReadWrite => {
                if !self.rw.can_write(&path.to_str()) {
                    return Err(VerySimpleError::FileCannotWrite);
                }
            }
        }

        if !vsfs::exists(&mut self.disk, path) {
            return Err(VerySimpleError::FileNotExist);
        }

        // 打开文件
        let id = self.rw.open(0, &path.to_str(), mode);


        update_access_time(&mut self.disk, path)
            .map_err(|err| VerySimpleError::VSFSError(err))?;

        // 返回文件
        Ok(VerySimpleFile {
            path: path.clone(),
            mode,
            position: 0,
            id,
        })
    }

    fn description(&mut self, file: &Self::File) -> Result<Self::FileDescription, Self::Error> {
        update_access_time(&mut self.disk, file.path())
            .map_err(|err| VerySimpleError::VSFSError(err))?;

        let inode = vsfs::get_inode_by_path(&self.disk, &file.path)
            .ok_or(VerySimpleError::UnknownError)?;

        Ok(VerySimpleFileDescription {
            inode: inode.clone(),
            name: file.path.current().unwrap().clone(),
        })
    }

    fn close(&mut self, file: Self::File) -> Result<(), Self::Error> {
        if self.rw.already_open(file.id) {
            return Err(VerySimpleError::FileNotOpen);
        }

        self.rw.close(file.id);
        Ok(())
    }

    fn read(&mut self, file: &mut Self::File, buf: &mut [u8]) -> Result<usize, Self::Error> {

        let mode = self.rw.access_mode(file.id)
            .ok_or(VerySimpleError::FileNotOpen)?;

        if mode != file.mode {
            return Err(VerySimpleError::AccessError);
        }

        let inode = vsfs::get_inode_by_path(&self.disk, &file.path)
            .ok_or(VerySimpleError::FileNotExist)?;

        let len = min(buf.len(), inode.size as usize - file.position);

        vsfs::read_file(&self.disk, &file.path, file.position, &mut buf[..len])
            .ok()
            .ok_or(VerySimpleError::UnknownError)?;

        file.position += len;

        update_access_time(&mut self.disk, file.path())
            .map_err(|err| VerySimpleError::VSFSError(err))?;

        Ok(len)
    }

    fn write(&mut self, file: &mut Self::File, buf: &[u8]) -> Result<usize, Self::Error> {
        let mode = self.rw.access_mode(file.id)
            .ok_or(VerySimpleError::FileNotOpen)?;

        if mode != file.mode || mode == Read {
            return Err(VerySimpleError::AccessError);
        }

        vsfs::write_file(&mut self.disk, &file.path, file.position, &buf)
            .map_err(|err| VerySimpleError::VSFSError(err))?;

        file.position += buf.len();

        update_modify_time(&mut self.disk, file.path())
            .map_err(|err| VerySimpleError::VSFSError(err))?;

        Ok(buf.len())
    }

    fn list(&mut self, path: &Path) -> Result<Vec<Self::FileDescription>, Self::Error> {
        let dir = vsfs::get_dir(&self.disk, path)
            .map_err(|err| VerySimpleError::VSFSError(err))?;

        let mut fds = Vec::new();

        for entry in dir.iter() {
            let path = path.clone().move_push(entry.name.clone());
            let inode = vsfs::get_inode_by_path(&self.disk, &path)
                .ok_or(VerySimpleError::UnknownError)?;

            fds.push(VerySimpleFileDescription {
                inode: inode.clone(),
                name: entry.name.clone(),
            })
        }

        update_access_time(&mut self.disk, path)
            .map_err(|err| VerySimpleError::VSFSError(err))?;

        Ok(fds)
    }

    fn mkdir(&mut self, path: &Path) -> Result<(), Self::Error> {
        let name = path.current()
            .ok_or(VerySimpleError::InvalidPath)?;
        let path = path.clone().parent()
            .ok_or(VerySimpleError::InvalidPath)?;

        vsfs::create_dir(&mut self.disk, &path, name)
            .map_err(|err| VerySimpleError::VSFSError(err))
    }

    fn rmdir(&mut self, path: &Path) -> Result<(), Self::Error> {
        vsfs::delete_dir(&mut self.disk, &path)
            .map_err(|err| VerySimpleError::VSFSError(err))
    }

    fn exists(&mut self, path: &Path) -> Result<bool, Self::Error> {
        let res = Ok(vsfs::exists(&self.disk, &path));
        update_access_time(&mut self.disk, path)
            .map_err(|err| VerySimpleError::VSFSError(err))?;
        res
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

    #[test]
    fn test_vfs_open() {
        let mut disk = Disk::new();
        let mut fs = VerySimpleFileSystem::new(&mut disk);
        fs.init().unwrap();

        let path = Path::from_str("/test.txt").unwrap();

        fs.create_file(&path).unwrap();

        let path = Path::from_str("/test.txt").unwrap();
        let file = fs.open(&path, AccessMode::ReadWrite).unwrap();

        assert_eq!(file.path().clone(), path);
        assert_eq!(file.position(), 0);
        assert_eq!(file.mode, AccessMode::ReadWrite);
    }

    #[test]
    fn test_rw() {
        let mut disk = Disk::new();
        let mut fs = VerySimpleFileSystem::new(&mut disk);
        fs.init().unwrap();


        let path = Path::from_str("/test.txt").unwrap();
        fs.create_file(&path).unwrap();

        let mut file = fs.open(&path, AccessMode::ReadWrite).unwrap();

        let mut buf = vec![0u8; 10020];
        for i in 0..buf.len() {
            buf[i] = i as u8;
        }

        fs.write(&mut file, &buf).unwrap();

        assert_eq!(file.position(), 10020);

        let mut read_buf = vec![0u8; 10020];
        file.set_position(0);
        fs.read(&mut file, &mut read_buf).unwrap();

        assert_eq!(buf, read_buf);
        assert_eq!(file.position(), 10020);

        let fd = fs.description(&file).unwrap();
        assert_eq!(fd.size(), 10020);
    }


    #[test]
    fn test_dir_list() {
        let mut disk = Disk::new();
        let mut fs = VerySimpleFileSystem::new(&mut disk);
        fs.init().unwrap();


        let path = Path::from_str("/test.txt").unwrap();
        fs.create_file(&path).unwrap();


        let fds = fs.list(&Path::root()).unwrap();
        assert_eq!(fds.len(), 1);
        assert_eq!(fds[0].is_dir(), false);
        assert_eq!(fds[0].name(), "test.txt");
    }

    #[test]
    fn test_mkdir() {
        let mut disk = Disk::new();
        let mut fs = VerySimpleFileSystem::new(&mut disk);
        fs.init().unwrap();

        let path = Path::from_str("/test").unwrap();
        fs.mkdir(&path).unwrap();

        let fds = fs.list(&Path::root()).unwrap();
        assert_eq!(fds.len(), 1);
        assert_eq!(fds[0].is_dir(), true);
        assert_eq!(fds[0].name(), "test");

        let path = Path::from_str("/test2").unwrap();
        fs.mkdir(&path).unwrap();

        let fds = fs.list(&Path::root()).unwrap();
        assert_eq!(fds.len(), 2);
        assert_eq!(fds[0].is_dir(), true);
        assert_eq!(fds[0].name(), "test");
        assert_eq!(fds[1].is_dir(), true);
        assert_eq!(fds[1].name(), "test2");


        let path = Path::from_str("/test/test3").unwrap();
        fs.mkdir(&path).unwrap();

        let fds = fs.list(&Path::root()).unwrap();
        assert_eq!(fds.len(), 2);
        assert_eq!(fds[0].is_dir(), true);
        assert_eq!(fds[0].name(), "test");
        assert_eq!(fds[1].is_dir(), true);
        assert_eq!(fds[1].name(), "test2");

        let fds = fs.list(&Path::root().move_push("test".to_string())).unwrap();
        assert_eq!(fds.len(), 1);
        assert_eq!(fds[0].is_dir(), true);
        assert_eq!(fds[0].name(), "test3");
    }
}