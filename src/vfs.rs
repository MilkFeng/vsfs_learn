use std::error::Error;
use std::fmt::Debug;
use crate::path::Path;

use crate::rw::AccessMode;

pub trait VirtualFile: Debug {
    fn path(&self) -> &Path;
    fn mode(&self) -> AccessMode;
    fn position(&self) -> usize;
    fn set_position(&mut self, pos: usize);
}

pub trait VirtualFileDescription: Debug {
    fn is_dir(&self) -> bool;
    fn name(&self) -> &str;
    fn ctime(&self) -> u64;
    fn mtime(&self) -> u64;
    fn size(&self) -> usize;
}

pub trait VirtualFileSystem {
    type File: VirtualFile;
    type Error: Error;
    type FileDescription: VirtualFileDescription;

    fn init(&mut self) -> Result<(), Self::Error>;

    fn create_file(&mut self, path: &Path) -> Result<Self::FileDescription, Self::Error>;
    fn delete_file(&mut self, path: &Path) -> Result<(), Self::Error>;

    fn open(&mut self, path: &Path, mode: AccessMode) -> Result<Self::File, Self::Error>;
    fn description(&mut self, file: &Self::File) -> Result<Self::FileDescription, Self::Error>;
    fn close(&mut self, file: Self::File) -> Result<(), Self::Error>;
    fn read(&mut self, file: &mut Self::File, buf: &mut [u8]) -> Result<usize, Self::Error>;
    fn write(&mut self, file: &mut Self::File, buf: &[u8]) -> Result<usize, Self::Error>;

    fn list(&mut self, path: &Path) -> Result<Vec<Self::FileDescription>, Self::Error>;
    fn mkdir(&mut self, path: &Path) -> Result<(), Self::Error>;
    fn rmdir(&mut self, path: &Path) -> Result<(), Self::Error>;

    fn exists(&mut self, path: &Path) -> Result<bool, Self::Error>;
}