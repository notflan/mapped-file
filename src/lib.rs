
use libc::{
    mmap,
    MAP_FAILED,
};

use std::{
    ops,
    mem,
    ptr,
};

mod uniq;
use uniq::UniqueSlice;

mod flags;
pub use flags::*;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
struct MappedSlice(UniqueSlice<u8>);

impl ops::Drop for MappedSlice
{
    #[inline]
    fn drop(&mut self) 
    {
	unsafe {
            libc::munmap(self.0.as_mut_ptr() as *mut _, self.0.len());
	}
    }
}

/// A memory mapping over file `T`.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct MappedFile<T>
{
    file: T,
    map: MappedSlice,
}

//TODO: continue copying from the `TODO` line in `utf8encode/src/mmap.rs`
