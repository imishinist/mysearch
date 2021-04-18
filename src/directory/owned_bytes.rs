use std::convert::TryInto;
use std::fmt::Formatter;
use std::ops::{Deref, Range};
use std::sync::Arc;
use std::{fmt, io, mem};

use stable_deref_trait::StableDeref;

use crate::FileHandle;

#[derive(Clone)]
pub struct OwnedBytes {
    data: &'static [u8],
    box_stable_deref: Arc<dyn Deref<Target = [u8]> + Sync + Send>,
}

impl FileHandle for OwnedBytes {
    fn read_bytes(&self, range: Range<usize>) -> io::Result<OwnedBytes> {
        Ok(self.slice(range))
    }
}

impl OwnedBytes {
    pub fn empty() -> Self {
        OwnedBytes::new(&[][..])
    }

    pub fn new<T: StableDeref + Deref<Target = [u8]> + 'static + Send + Sync>(
        data_holder: T,
    ) -> Self {
        let box_stable_deref = Arc::new(data_holder);
        let bytes: &[u8] = box_stable_deref.as_ref();
        let data = unsafe { mem::transmute::<_, &'static [u8]>(bytes.deref()) };
        OwnedBytes {
            box_stable_deref,
            data,
        }
    }

    pub fn slice(&self, range: Range<usize>) -> Self {
        OwnedBytes {
            data: &self.data[range],
            box_stable_deref: self.box_stable_deref.clone(),
        }
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[u8] {
        self.data
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.as_slice().is_empty()
    }

    pub fn split(self, split_len: usize) -> (OwnedBytes, OwnedBytes) {
        let right_box_stable_deref = self.box_stable_deref.clone();
        let left = OwnedBytes {
            data: &self.data[..split_len],
            box_stable_deref: self.box_stable_deref,
        };
        let right = OwnedBytes {
            data: &self.data[split_len..],
            box_stable_deref: right_box_stable_deref,
        };
        (left, right)
    }

    #[inline(always)]
    pub fn advance(&mut self, advance_len: usize) {
        self.data = &self.data[advance_len..]
    }

    pub fn read_u8(&mut self) -> u8 {
        assert!(!self.is_empty());

        let byte = self.as_slice()[0];
        self.advance(1);
        byte
    }

    pub fn read_u64(&mut self) -> u64 {
        assert!(self.len() >= 8);

        let octlet: [u8; 8] = self.as_slice()[..8].try_into().unwrap();
        self.advance(8);
        u64::from_le_bytes(octlet)
    }
}

impl fmt::Debug for OwnedBytes {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let bytes_truncated: &[u8] = if self.len() > 8 {
            &self.as_slice()[..10]
        } else {
            self.as_slice()
        };
        write!(f, "OwnedBytes({:?}, len={})", bytes_truncated, self.len())
    }
}

impl Deref for OwnedBytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl AsRef<[u8]> for OwnedBytes {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl io::Read for OwnedBytes {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let read_len = {
            let data = self.as_slice();
            if data.len() >= buf.len() {
                let buf_len = buf.len();
                buf.copy_from_slice(&data[..buf_len]);
                buf.len()
            } else {
                let data_len = data.len();
                buf[..data_len].copy_from_slice(data);
                data_len
            }
        };
        self.advance(read_len);
        Ok(read_len)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        let read_len = {
            let data = self.as_slice();
            buf.extend(data);
            data.len()
        };
        self.advance(read_len);
        Ok(read_len)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        let read_len = self.read(buf)?;
        if read_len != buf.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "failed to fill whole buffer",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::OwnedBytes;
    use std::io::{self, Read};

    #[test]
    fn test_owned_bytes_debug() {
        let short_bytes = OwnedBytes::new(b"abcd".as_ref());
        assert_eq!(
            format!("{:?}", short_bytes),
            "OwnedBytes([97, 98, 99, 100], len=4)"
        );

        let long_bytes = OwnedBytes::new(b"abcdefghijklmnopq".as_ref());
        assert_eq!(
            format!("{:?}", long_bytes),
            "OwnedBytes([97, 98, 99, 100, 101, 102, 103, 104, 105, 106], len=17)"
        );
    }

    #[test]
    fn test_owned_bytes_read() -> io::Result<()> {
        let mut bytes = OwnedBytes::new(b"abcdefghiklmnopqrstuvwxyz".as_ref());
        {
            let mut buf = [0u8; 5];
            bytes.read_exact(&mut buf[..]).unwrap();
            assert_eq!(&buf, b"abcde");
            assert_eq!(bytes.as_slice(), b"fghiklmnopqrstuvwxyz")
        }
        {
            let mut buf = [0u8; 2];
            bytes.read_exact(&mut buf[..]).unwrap();
            assert_eq!(&buf, b"fg");
            assert_eq!(bytes.as_slice(), b"hiklmnopqrstuvwxyz")
        }
        Ok(())
    }

    #[test]
    fn test_owned_bytes_read_right_at_the_end() -> io::Result<()> {
        let mut bytes = OwnedBytes::new(b"abcde".as_ref());
        let mut buf = [0u8; 5];
        assert_eq!(bytes.read(&mut buf[..]).unwrap(), 5);
        assert_eq!(&buf, b"abcde");
        assert_eq!(bytes.as_slice(), b"");
        assert_eq!(bytes.read(&mut buf[..]).unwrap(), 0);
        assert_eq!(&buf, b"abcde");
        Ok(())
    }

    #[test]
    fn test_owned_bytes_read_incomplete() -> io::Result<()> {
        let mut bytes = OwnedBytes::new(b"abcde".as_ref());
        let mut buf = [0u8; 7];
        assert_eq!(bytes.read(&mut buf[..]).unwrap(), 5);
        assert_eq!(&buf[..5], b"abcde");
        assert_eq!(bytes.read(&mut buf[..]).unwrap(), 0);
        Ok(())
    }

    #[test]
    fn test_owned_bytes_read_to_end() -> io::Result<()> {
        let mut bytes = OwnedBytes::new(b"abcde".as_ref());
        let mut buf = Vec::new();
        bytes.read_to_end(&mut buf)?;
        assert_eq!(buf.as_slice(), b"abcde".as_ref());
        Ok(())
    }

    #[test]
    fn test_owned_bytes_read_u8() -> io::Result<()> {
        let mut bytes = OwnedBytes::new(b"\xFF".as_ref());
        assert_eq!(bytes.read_u8(), u8::MAX);
        assert_eq!(bytes.len(), 0);
        Ok(())
    }

    #[test]
    fn test_owned_bytes_read_u64() -> io::Result<()> {
        let mut bytes = OwnedBytes::new(b"\0\xFF\xFF\xFF\xFF\xFF\xFF\xFF".as_ref());
        assert_eq!(bytes.read_u64(), u64::MAX - 255);
        assert_eq!(bytes.len(), 0);
        Ok(())
    }

    #[test]
    fn test_owned_bytes_split() {
        let bytes = OwnedBytes::new(b"abcdefghi".as_ref());
        let (left, right) = bytes.split(3);
        assert_eq!(left.as_slice(), b"abc");
        assert_eq!(right.as_slice(), b"defghi");
    }

    #[test]
    fn test_owned_bytes_split_boundary() {
        let bytes = OwnedBytes::new(b"abcdefghi".as_ref());
        {
            let (left, right) = bytes.clone().split(0);
            assert_eq!(left.as_slice(), b"");
            assert_eq!(right.as_slice(), b"abcdefghi");
        }
        {
            let (left, right) = bytes.split(9);
            assert_eq!(left.as_slice(), b"abcdefghi");
            assert_eq!(right.as_slice(), b"");
        }
    }
}
