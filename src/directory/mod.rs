use std::io::{BufWriter, Write};

mod file_slice;
mod owned_bytes;
mod directory;

pub use file_slice::*;
pub use owned_bytes::*;
pub use directory::*;


pub trait HasLen {
    fn len(&self) -> usize;
}

pub type WritePtr = BufWriter<Box<dyn Write>>;