use std::error::Error;
use crate::path::Path;

use crate::rw::AccessMode;

pub trait VirtualFile {
    fn path(&self) -> &Path;
    fn mode(&self) -> AccessMode;
    fn position(&self) -> usize;
    fn set_position(&mut self, pos: usize);
}

pub trait VirtualFileDescription {
    fn is_dir(&self) -> bool;
    fn name(&self) -> &str;
    fn ctime(&self) -> u64;
    fn mtime(&self) -> u64;
    fn size(&self) -> u64;
}

pub trait VirtualFileSystem {
    type File: VirtualFile;
    type Error: Error;
    type FileDescription: VirtualFileDescription;

    fn init(&mut self) -> Result<(), Self::Error>;

    fn open(&mut self, path: &Path, mode: AccessMode) -> Result<Self::File, Self::Error>;
    fn close(&mut self, file: Self::File) -> Result<(), Self::Error>;
    fn read(&self, file: &mut Self::File, buf: &mut [u8]) -> Result<usize, Self::Error>;
    fn write(&mut self, file: &mut Self::File, buf: &[u8]) -> Result<usize, Self::Error>;

    fn list(&self, path: &Path) -> Result<Vec<Self::FileDescription>, Self::Error>;
    fn mkdir(&mut self, path: &Path) -> Result<(), Self::Error>;
    fn rmdir(&mut self, path: &Path) -> Result<(), Self::Error>;

    fn exists(&self, path: &Path) -> Result<bool, Self::Error>;
}