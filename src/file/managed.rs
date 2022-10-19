//! Represents a managed `RawFd`.
//! This will `close()` its contained `RawFd` on drop.
//!
//! Can be useful for OS operations on file descriptors without leaking open fds.
use super::*;
use std::{
    ops,
};
use libc::{
    dup, dup2,
    close,
};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ManagedFD(UnmanagedFD);

impl Clone for ManagedFD {
    fn clone(&self) -> Self {
	Self(unsafe { UnmanagedFD::new_unchecked( c_try!(dup(self.0.get()) => if |x| x < 0; "dup(): failed to duplicate file descriptor {}", self.0.get()) ) })
    }
    fn clone_from(&mut self, source: &Self) {
	c_try!(dup2(self.0.get(), source.0.get()) => -1; "dup2(): failed to set file descriptor {} to alias {}", self.0.get(), source.0.get());
    }
}

impl ops::Drop for ManagedFD
{
    fn drop(&mut self) {
	unsafe {
	    close(self.0.get());
	}
    }
}

impl AsRawFd for ManagedFD
{
    #[inline] 
    fn as_raw_fd(&self) -> RawFd {
	self.0.get()
    }
}

impl FromRawFd for ManagedFD
{
    #[inline] 
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
	Self(UnmanagedFD::new_unchecked(fd))
    }
}

impl IntoRawFd for ManagedFD
{
    #[inline] 
    fn into_raw_fd(self) -> RawFd {
	let raw = self.0.get();
	std::mem::forget(self);
	raw
    }
}

impl From<ManagedFD> for std::fs::File
{
    #[inline] 
    fn from(from: ManagedFD) -> Self
    {
	unsafe {
	    Self::from_raw_fd(from.into_raw_fd())
	}
    }
}


//TODO: implement the rest of ManagedFD from `memfd` module in `utf8encode`
