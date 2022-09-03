//! Provides physical in-memory file descriptors.
//!
//! This can be useful for temporary buffers where a file descriptor is required.
//! Huge-pages can also be used for this memory.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct MemoryFile(ManagedFD);

mod hugetlb;
//TODO: implement `memfd` (and its hugetlb interface, extracted to `hugetlb.rs`) from `utf8encode`.
