use std::io::{BufWriter, Write};

mod directory;
mod file_slice;
mod owned_bytes;
mod ram_directory;

pub use directory::*;
pub use file_slice::*;
pub use owned_bytes::*;
pub use ram_directory::*;

pub trait HasLen {
    fn len(&self) -> usize;
}

pub type WritePtr = BufWriter<Box<dyn Write>>;
