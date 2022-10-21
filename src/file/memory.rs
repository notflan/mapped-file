//! Provides physical in-memory file descriptors.
//!
//! This can be useful for temporary buffers where a file descriptor is required.
//! Huge-pages can also be used for this memory.
use super::*;
use libc::{
    c_uint,
    memfd_create,
    MFD_CLOEXEC,
    MFD_HUGETLB,

    ftruncate,
};
use std::{
    ffi::CStr,
    borrow::{
	Borrow,
	BorrowMut,
    },
    ops,
};
use hugetlb::{
    MapHugeFlag,
    HugePage,
};

static UNNAMED: &'static CStr = unsafe {
    CStr::from_bytes_with_nul_unchecked(b"<unnamed memory file>\0")
};

const DEFAULT_FLAGS: c_uint = MFD_CLOEXEC;

#[inline(always)]
//XXX: Is the static bound required here?
/// Create a raw, unmanaged, memory file with these flags and this name.
///
/// # Safety
/// The reference obtained by `name` must not move as long as the `Ok()` result is alive.
pub unsafe fn create_raw(name: impl AsRef<CStr>, flags: c_uint) -> io::Result<UnmanagedFD> 
{
    UnmanagedFD::new_raw(memfd_create(name.as_ref().as_ptr(), flags)).ok_or_else(|| io::Error::last_os_error())
}

/// A physical-memory backed file
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct MemoryFile(ManagedFD);

/// A named, physical-memory backed file
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NamedMemoryFile(Box<CStr>, MemoryFile);

impl Borrow<MemoryFile> for NamedMemoryFile
{
    #[inline] 
    fn borrow(&self) -> &MemoryFile {
	&self.1
    }
}
impl BorrowMut<MemoryFile> for NamedMemoryFile
{
    #[inline] 
    fn borrow_mut(&mut self) -> &mut MemoryFile {
	&mut self.1
    }
}
impl ops::DerefMut for NamedMemoryFile
{
    fn deref_mut(&mut self) -> &mut Self::Target {
	&mut self.1
    }
}
impl ops::Deref for NamedMemoryFile
{
    type Target = MemoryFile;
    #[inline] 
    fn deref(&self) -> &Self::Target {
	&self.1
    }
}

//TODO: impl `MemoryFile` (memfd_create() fd wrapper)
impl MemoryFile
{
    /// Create a new, empty, memory file with no name and no flags.
    pub fn new() -> io::Result<Self>
    {
	let managed = unsafe {
	    match memfd_create(UNNAMED.as_ptr(), DEFAULT_FLAGS) {
		-1 => return Err(io::Error::last_os_error()),
		fd => ManagedFD::take_unchecked(fd),
	    }
	};
	Ok(Self(managed))
    }
    #[inline] 
    pub fn resize(&mut self, value: usize) -> io::Result<()>
    {
	if 0 == unsafe { ftruncate(self.as_raw_fd(), value.try_into().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?) } {
	    Ok(())
	} else {
	    Err(io::Error::last_os_error())
	}
    }
    
    pub fn with_hugetlb(hugetlb: MapHugeFlag) -> io::Result<Self>
    {
	unsafe { create_raw(UNNAMED, DEFAULT_FLAGS | (hugetlb.get_mask() as c_uint)) }
	.map(ManagedFD::take)
	    .map(Self)
    }

    pub fn with_size(size: usize) -> io::Result<Self>
    {
	let mut this = Self(unsafe { create_raw(UNNAMED, DEFAULT_FLAGS) }.map(ManagedFD::take)?);
	this.resize(size)?;
	Ok(this)
    }

    #[inline] 
    pub fn with_size_hugetlb(size: usize, hugetlb: MapHugeFlag) -> io::Result<Self>
    {
	let mut this = Self::with_hugetlb(hugetlb)?;
	this.resize(size)?;
	Ok(this)
    }
}

fn alloc_cstring(string: &str) -> std::ffi::CString
{
    #[cold]
    fn _contains_nul(mut bytes: Vec<u8>) -> std::ffi::CString
    {
	// SAFETY: We know this will only be called if byte `0` is in `bytes` (**before** the final element)
	let len = unsafe {
	    memchr::memchr(0, &bytes[..]).unwrap_unchecked()
	};
	bytes.truncate(len);
	// SAFETY: We have truncated the vector to end on the *first* instance of the `0` byte in `bytes`.
	unsafe {
	    std::ffi::CString::from_vec_with_nul_unchecked(bytes)
	}
    }
    let mut bytes = Vec::with_capacity(string.len()+1);
    bytes.extend_from_slice(string.as_bytes());
    bytes.push(0);
    match std::ffi::CString::from_vec_with_nul(bytes) {
	Ok(v) => v,
	Err(cn) => {
	    _contains_nul(cn.into_bytes())
	}
    }
}

impl NamedMemoryFile
{
    #[inline] 
    pub fn new(name: impl AsRef<str>) -> io::Result<Self>
    {
	let name: Box<CStr> = alloc_cstring(name.as_ref()).into();
	let managed = unsafe {
	    match memfd_create(name.as_ptr(), DEFAULT_FLAGS) {
		-1 => return Err(io::Error::last_os_error()),
		fd => ManagedFD::take_unchecked(fd),
	    }
	};
	Ok(Self(name, MemoryFile(managed)))
    }

    pub fn with_hugetlb(name: impl AsRef<str>, hugetlb: MapHugeFlag) -> io::Result<Self>
    {
	let name: Box<CStr> = alloc_cstring(name.as_ref()).into();
	let memfd = MemoryFile(unsafe { create_raw(&name, DEFAULT_FLAGS | (hugetlb.get_mask() as c_uint)) }
			       .map(ManagedFD::take)?);
	Ok(Self(name, memfd))
    }

    pub fn with_size(name: impl AsRef<str>, size: usize) -> io::Result<Self>
    {
	let name: Box<CStr> = alloc_cstring(name.as_ref()).into();
	let mut this = MemoryFile(unsafe { create_raw(&name, DEFAULT_FLAGS) }.map(ManagedFD::take)?);
	this.resize(size)?;
	Ok(Self(name, this))
    }

    #[inline] 
    pub fn with_size_hugetlb(name: impl AsRef<str>, size: usize, hugetlb: MapHugeFlag) -> io::Result<Self>
    {
	let mut this = Self::with_hugetlb(name, hugetlb)?;
	this.resize(size)?;
	Ok(this)
    }
}

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
