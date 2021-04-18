use std::{fmt, io};
use std::path::Path;
use crate::{FileHandle, FileSlice, WritePtr};

pub trait Directory: DirectoryClone + fmt::Debug + Send + Sync + 'static {
    fn get_file_handle(&self, path: &Path) -> io::Result<Box<dyn FileHandle>>;

    fn open_read(&self, path: &Path) -> io::Result<FileSlice> {
        let file_handle = self.get_file_handle(path)?;
        Ok(FileSlice::new(file_handle))
    }

    fn exists(&self, path: &Path) -> io::Result<bool>;

    fn open_write(&self, path: &Path) -> io::Result<WritePtr>;
}

pub trait DirectoryClone {
    fn box_clone(&self) -> Box<dyn Directory>;
}

impl<T> DirectoryClone for T
where
T: 'static + Directory + Clone
{
    fn box_clone(&self) -> Box<dyn Directory> {
        Box::new(self.clone())
    }
}