use std::fmt;
use std::fmt::Formatter;
use std::io;
use std::ops::{Deref, Range};
use std::sync::{Arc, Weak};

use stable_deref_trait::StableDeref;

use crate::directory::OwnedBytes;
use crate::HasLen;

pub type ArcBytes = Arc<dyn Deref<Target = [u8]> + Send + Sync + 'static>;
pub type WeakBytes = Weak<dyn Deref<Target = [u8]> + Send + Sync + 'static>;

pub trait FileHandle: 'static + Send + Sync + HasLen + fmt::Debug {
    fn read_bytes(&self, range: Range<usize>) -> io::Result<OwnedBytes>;
}

impl FileHandle for &'static [u8] {
    fn read_bytes(&self, range: Range<usize>) -> io::Result<OwnedBytes> {
        let bytes = &self[range];
        Ok(OwnedBytes::new(bytes))
    }
}

impl<T: Deref<Target = [u8]>> HasLen for T {
    fn len(&self) -> usize {
        self.deref().len()
    }
}

impl<B> From<B> for FileSlice
where
    B: StableDeref + Deref<Target = [u8]> + 'static + Send + Sync,
{
    fn from(bytes: B) -> Self {
        FileSlice::new(Box::new(OwnedBytes::new(bytes)))
    }
}

#[derive(Clone)]
pub struct FileSlice {
    data: Arc<dyn FileHandle>,
    range: Range<usize>,
}

impl fmt::Debug for FileSlice {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "FileSlice({:?}, {:?})", &self.data, self.range)
    }
}

impl FileSlice {
    pub fn new(file_handle: Box<dyn FileHandle>) -> Self {
        let num_bytes = file_handle.len();
        FileSlice::new_with_num_bytes(file_handle, num_bytes)
    }

    pub fn new_with_num_bytes(file_handle: Box<dyn FileHandle>, num_bytes: usize) -> Self {
        FileSlice {
            data: Arc::from(file_handle),
            range: 0..num_bytes,
        }
    }

    pub fn slice(&self, bytes_range: Range<usize>) -> FileSlice {
        assert!(bytes_range.end <= self.len());

        FileSlice {
            data: self.data.clone(),
            range: self.range.start + bytes_range.start..self.range.start + bytes_range.end,
        }
    }

    pub fn empty() -> FileSlice {
        const EMPTY_SLICE: &[u8] = &[];
        FileSlice::from(EMPTY_SLICE)
    }

    pub fn read_bytes(&self) -> io::Result<OwnedBytes> {
        self.data.read_bytes(self.range.clone())
    }

    pub fn read_bytes_slice(&self, range: Range<usize>) -> io::Result<OwnedBytes> {
        assert!(
            range.end <= self.len(),
            "end of requested range exeeds the fileslice length({} > {})",
            range.end,
            self.len()
        );
        self.data
            .read_bytes(self.range.start + range.start..self.range.start + range.end)
    }

    pub fn split(self, left_len: usize) -> (FileSlice, FileSlice) {
        let left = self.slice_to(left_len);
        let right = self.slice_from(left_len);
        (left, right)
    }

    pub fn split_from_end(self, right_len: usize) -> (FileSlice, FileSlice) {
        let left_len = self.len() - right_len;
        self.split(left_len)
    }

    pub fn slice_from(&self, from_offset: usize) -> FileSlice {
        self.slice(from_offset..self.len())
    }

    pub fn slice_to(&self, to_offset: usize) -> FileSlice {
        self.slice(0..to_offset)
    }
}

impl FileHandle for FileSlice {
    fn read_bytes(&self, range: Range<usize>) -> io::Result<OwnedBytes> {
        self.read_bytes_slice(range)
    }
}

impl HasLen for FileSlice {
    fn len(&self) -> usize {
        self.range.len()
    }
}

#[cfg(test)]
mod tests {
    use super::{FileHandle, FileSlice};
    use crate::HasLen;
    use std::io;

    #[test]
    fn test_file_slice() -> io::Result<()> {
        let file_slice = FileSlice::new(Box::new(b"abcdef".as_ref()));
        assert_eq!(file_slice.len(), 6);
        assert_eq!(file_slice.slice_from(2).read_bytes()?.as_slice(), b"cdef");
        assert_eq!(file_slice.slice_to(2).read_bytes()?.as_slice(), b"ab");
        assert_eq!(
            file_slice
                .slice_from(1)
                .slice_to(2)
                .read_bytes()?
                .as_slice(),
            b"bc"
        );
        {
            let (left, right) = file_slice.clone().split(0);
            assert_eq!(left.read_bytes()?.as_slice(), b"");
            assert_eq!(right.read_bytes()?.as_slice(), b"abcdef");
        }
        {
            let (left, right) = file_slice.clone().split(2);
            assert_eq!(left.read_bytes()?.as_slice(), b"ab");
            assert_eq!(right.read_bytes()?.as_slice(), b"cdef");
        }
        {
            let (left, right) = file_slice.clone().split_from_end(0);
            assert_eq!(left.read_bytes()?.as_slice(), b"abcdef");
            assert_eq!(right.read_bytes()?.as_slice(), b"");
        }
        {
            let (left, right) = file_slice.clone().split_from_end(2);
            assert_eq!(left.read_bytes()?.as_slice(), b"abcd");
            assert_eq!(right.read_bytes()?.as_slice(), b"ef");
        }
        Ok(())
    }

    #[test]
    fn test_file_slice_trait_slice_len() {
        let blop: &'static [u8] = b"abc";
        let owned_bytes: Box<dyn FileHandle> = Box::new(blop);
        assert_eq!(owned_bytes.len(), 3);
    }

    #[test]
    fn test_slice_simple_read() -> io::Result<()> {
        let slice = FileSlice::new(Box::new(&b"abcdef"[..]));
        assert_eq!(slice.len(), 6);
        assert_eq!(slice.read_bytes()?.as_ref(), b"abcdef");
        assert_eq!(slice.slice(1..4).read_bytes()?.as_ref(), b"bcd");
        Ok(())
    }

    #[test]
    fn test_slice_read_slice() -> io::Result<()> {
        let slice_deref = FileSlice::new(Box::new(&b"abcdef"[..]));
        assert_eq!(slice_deref.read_bytes_slice(1..4)?.as_ref(), b"bcd");
        Ok(())
    }
}
