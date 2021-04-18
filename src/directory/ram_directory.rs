
use crate::{FileHandle, FileSlice, HasLen, Directory, WritePtr};
use std::path::{PathBuf, Path};
use std::collections::HashMap;
use std::{io, fmt};
use std::sync::{RwLock, Arc};
use std::fmt::Formatter;
use std::io::{Write, Cursor, Seek, SeekFrom, BufWriter};


struct VecWriter {
    path: PathBuf,
    shared_directory: RAMDirectory,
    data: Cursor<Vec<u8>>,
    is_flushed: bool,
}

impl VecWriter {
    fn new(path_buf: PathBuf, shared_directory: RAMDirectory) -> Self {
        VecWriter {
            path: path_buf,
            data: Cursor::new(Vec::new()),
            shared_directory,
            is_flushed: true,
        }
    }
}

impl Drop for VecWriter {
    fn drop(&mut self) {
        if !self.is_flushed {
            panic!("You forgot to flush {:?} before its writer got Drop. Do not rely on dop.",
                self.path
            )
        }
    }
}

impl Seek for VecWriter {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.data.seek(pos)
    }
}

impl Write for VecWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.is_flushed = false;
        self.data.write_all(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.is_flushed = true;
        let mut fs = self.shared_directory.fs.write().unwrap();
        fs.write(self.path.clone(), self.data.get_ref());
        Ok(())
    }
}

#[derive(Default)]
struct InnerDirectory {
    fs: HashMap<PathBuf, FileSlice>,
}

impl InnerDirectory {
    fn write(&mut self, path: PathBuf, data: &[u8]) -> bool {
        let data = FileSlice::from(data.to_vec());
        self.fs.insert(path, data).is_some()
    }

    fn open_read(&self, path: &Path) -> io::Result<FileSlice> {
        self.fs
            .get(path)
            .ok_or_else(|| io::ErrorKind::NotFound.into())
            .map(Clone::clone)
    }

    fn exists(&self, path: &Path) -> bool {
        self.fs.contains_key(path)
    }

    fn total_mem_usage(&self) -> usize {
        self.fs.values().map(|f| f.len()).sum()
    }
}


#[derive(Clone, Default)]
pub struct RAMDirectory {
    fs: Arc<RwLock<InnerDirectory>>
}

impl RAMDirectory {
    pub fn create() -> RAMDirectory {
        Self::default()
    }

    pub fn total_mem_usage(&self) -> usize {
        self.fs.read().unwrap().total_mem_usage()
    }

    pub fn persist(&self, dest: &dyn Directory) -> io::Result<()> {
        let wlock = self.fs.write().unwrap();
        for (path, file) in wlock.fs.iter() {
            let mut dest_wrt = dest.open_write(path)?;
            dest_wrt.write_all(file.read_bytes()?.as_slice())?;
            dest_wrt.flush()?;
        }
        Ok(())
    }
}

impl Directory for RAMDirectory {
    fn get_file_handle(&self, path: &Path) -> io::Result<Box<dyn FileHandle>> {
        let file_slice = self.open_read(path)?;
        Ok(Box::new(file_slice))
    }

    fn open_read(&self, path: &Path) -> io::Result<FileSlice> {
        self.fs.read().unwrap().open_read(path)
    }

    fn exists(&self, path: &Path) -> io::Result<bool> {
        Ok(self.fs.read().map_err(|_e| io::ErrorKind::NotFound)?.exists(path))
    }

    fn open_write(&self, path: &Path) -> io::Result<WritePtr> {
        let mut fs = self.fs.write().unwrap();
        let path_buf = PathBuf::from(path);
        let vec_writer = VecWriter::new(path_buf.clone(), self.clone());
        let exists = fs.write(path_buf.clone(), &[]);
        if exists {
            Err(io::ErrorKind::AlreadyExists.into())
        } else {
            Ok(BufWriter::new(Box::new(vec_writer)))
        }
    }

    fn atomic_read(&self, path: &Path) -> io::Result<Vec<u8>> {
        let bytes =
        self.open_read(path)?
            .read_bytes()?;
        Ok(bytes.as_slice().to_owned())
    }
}

impl fmt::Debug for RAMDirectory {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "RAMDirectory")
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use crate::{RAMDirectory, Directory};
    use std::io::Write;

    #[test]
    fn test_persist() {
        let path: &'static Path = Path::new("seq");
        let msg: &'static [u8] = b"sequential is the way";
        let directory = RAMDirectory::create();
        let mut wrt = directory.open_write(path).unwrap();
        assert!(wrt.write_all(msg).is_ok());
        assert!(wrt.flush().is_ok());

        let directory_copy = RAMDirectory::create();
        assert!(directory.persist(&directory_copy).is_ok());
        assert_eq!(directory_copy.atomic_read(path).unwrap(), msg);
    }
}