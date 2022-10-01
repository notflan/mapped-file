//! Provides a wrapper over `RawFd` that does not close it on drop.
//! This can be useful for aliasing file descriptors.
use super::*;

/// Represents a `RawFd` but does not provide any ownership of it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnmanagedFD(RawFd);

impl UnmanagedFD {
    #[inline] 
    pub fn new(alias: &(impl AsRawFd + ?Sized)) -> Self
    {
	Self(alias.as_raw_fd())
    }
}

impl From<RawFd> for UnmanagedFD
{
    #[inline] 
    fn from(from: RawFd) -> Self
    {
	debug_assert!(from >= 0, "Invalid file descriptor");
	Self(from)
    }
}

impl FromRawFd for UnmanagedFD
{
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
	Self(fd)
    }
}


impl AsRawFd for UnmanagedFD
{
    #[inline(always)] 
    fn as_raw_fd(&self) -> RawFd {
	self.0
    }
}

//TODO: implement a full version of the temporary struct `UnmanagedFD` from `utf8encode`
