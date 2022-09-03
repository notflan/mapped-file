//! Provides a wrapper over `RawFd` that does not close it on drop.
//! This can be useful for aliasing file descriptors.
use super::*;

/// Represents a `RawFd` but does not provide any ownership of it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnmanagedFD(RawFd);

//TODO: implement a full version of the temporary struct `UnmanagedFD` from `utf8encode`
