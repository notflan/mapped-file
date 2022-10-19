//! Provides physical in-memory file descriptors.
//!
//! This can be useful for temporary buffers where a file descriptor is required.
//! Huge-pages can also be used for this memory.
use super::*;
use libc::{
    memfd_create,
    MFD_CLOEXEC,
    MFD_HUGETLB,
};

/// A physical-memory backed file
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct MemoryFile(ManagedFD);

//TODO: impl `MemoryFile` (memfd_create() fd wrapper)

impl AsRawFd for MemoryFile
{
    #[inline] 
    fn as_raw_fd(&self) -> RawFd {
	self.0.as_raw_fd()
    }
}

impl FromRawFd for MemoryFile
{
    #[inline] 
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
	Self(ManagedFD::from_raw_fd(fd))
    }
}

impl IntoRawFd for MemoryFile
{
    #[inline]
    fn into_raw_fd(self) -> RawFd {
	self.0.into_raw_fd()
    }
}

impl From<MemoryFile> for ManagedFD
{
    #[inline] 
    fn from(from: MemoryFile) -> Self
    {
	from.0
    }
}

impl From<MemoryFile> for std::fs::File
{
    #[inline] 
    fn from(from: MemoryFile) -> Self
    {
	from.0.into()
    }
}

//TODO: implement `memfd` from `utf8encode`.
