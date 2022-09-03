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
pub struct ManagedFD(RawFd);

impl Clone for ManagedFD {
    fn clone(&self) -> Self {
	Self(c_try!(dup(self.0) => if |x| x < 0; "dup(): failed to duplicate file descriptor {}", self.0))
    }
    fn clone_from(&mut self, source: &Self) {
	c_try!(dup2(self.0, source.0) => -1; "dup2(): failed to set file descriptor {} to alias {}", self.0, source.0);
    }
}

impl ops::Drop for ManagedFD
{
    fn drop(&mut self) {
	unsafe {
	    close(self.0);
	}
    }
}

//TODO: implement the rest of ManagedFD from `memfd` module in `utf8encode`
