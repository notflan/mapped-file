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

//TODO: io::Read/io::Write impls for ManagedFD

impl ManagedFD
{
    #[inline] 
    pub const unsafe fn take_unchecked(fd: RawFd) -> Self
    {
	Self(UnmanagedFD::new_unchecked(fd))
    }

    /// Duplicate a file-descriptor, aliasing the open resource for the lifetime of the returned `ManagedFD`..
    #[inline]
    pub fn alias(file: &(impl AsRawFd + ?Sized)) -> io::Result<Self>
    {
	let r = unsafe { libc::dup(file.as_raw_fd()) };
	if let Some(r) = UnmanagedFD::new_raw(r) {
	    Ok(Self(r))
	} else {
	    Err(io::Error::last_os_error())
	}
    }

    #[inline] 
    pub const fn take_raw(fd: RawFd) -> Self
    {
	assert!(fd>=0, "Invalid file descriptor");
	unsafe {
	    Self::take_unchecked(fd)
	}
    }

    #[inline] 
    pub const fn take(fd: UnmanagedFD) -> Self
    {
	Self(fd)
    }

    #[inline]
    pub fn detach(self) -> UnmanagedFD
    {
	let v = self.0.clone();
	std::mem::forget(self);
	v
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
