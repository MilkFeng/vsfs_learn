use std::error::Error;
use std::path::Path;

/// 可以存储为文件
pub trait Savable where Self: Sized {
    /// 存储为文件
    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), impl Error>;
}


/// 可以从文件加载
pub trait Loadable where Self: Sized {
    /// 从文件加载
    fn load<P: AsRef<Path>>(path: P) -> Result<Box<Self>, impl Error>;
}