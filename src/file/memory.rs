//! Provides physical in-memory file descriptors.
//!
//! This can be useful for temporary buffers where a file descriptor is required.
//! Huge-pages can also be used for this memory.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct MemoryFile(ManagedFD);

//TODO: implement `memfd` from `utf8encode`.
