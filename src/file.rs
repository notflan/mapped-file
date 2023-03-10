//! Types for, and operations on file descriptors. Useful for mapping
use super::*;

/// Raw file-descriptor for standard input
pub const STDIN_FILENO: RawFd = libc::STDIN_FILENO;
/// Raw file-descriptor for standard output
pub const STDOUT_FILENO: RawFd = libc::STDOUT_FILENO;
/// Raw file-descriptor for standard error
pub const STDERR_FILENO: RawFd = libc::STDERR_FILENO;

mod raw;
use raw::*;

mod managed;
mod unmanaged;

pub use self::{
    managed::*,
    unmanaged::*,
};

pub mod memory;

#[derive(Debug)]
enum MaybeMappedInner<T>
{
    Raw(T),
    Copied(memory::MemoryFile),
}

impl<T: AsRawFd + io::Read> MaybeMappedInner<T>
{
    pub fn from_stat(mut file: T) -> io::Result<(Self, u64)>
    {
	use libc::fstat;
	let fd = file.as_raw_fd();
	let sz = unsafe {
	    let mut stat = std::mem::MaybeUninit::uninit();
	    if fstat(fd, stat.as_mut_ptr()) != 0 {
		let mut mem = memory::MemoryFile::new()?;
		let count = std::io::copy(&mut file, &mut mem)?;
		return Ok((Self::Copied(mem), count));
	    }
	    stat.assume_init().st_size & i64::MAX
	} as u64;
	Ok((Self::Raw(file), sz))
    }
}

impl<T: IntoRawFd> MaybeMappedInner<T>
{
    #[inline] 
    pub unsafe fn into_file(self) -> std::fs::File
    {
	let fd = match self {
	    Self::Raw(r) => r.into_raw_fd(),
	    Self::Copied(c) => c.into_raw_fd(),
	};
	
	FromRawFd::from_raw_fd(fd)
    }
}

impl<T> AsRawFd for MaybeMappedInner<T>
where T: AsRawFd
{
    #[inline] 
    fn as_raw_fd(&self) -> RawFd {
	match self {
	    Self::Copied(c) => c.as_raw_fd(),
	    Self::Raw(r) => r.as_raw_fd(),
	}
    }
}

/// Attempt to map a file, if it fails, copy that file into memory and map that.
///
/// # Returns
/// A map over the file, or a map over an in-memory copy of the file.
pub fn try_map_or_cloned<F: io::Read + AsRawFd + IntoRawFd>(file: F, perm: Perm, flags: impl MapFlags) -> io::Result<MappedFile<std::fs::File>>
{
    let (len, file) = {
	let (file, size) = MaybeMappedInner::from_stat(file)?;
	let size = usize::try_from(size).map_err(|_| io::Error::new(io::ErrorKind::Unsupported, "File size exceeds pointer word width"))?;
	(size, unsafe {
	    file.into_file() 
	})
    };
    MappedFile::new(file, len, perm, flags)
}

#[cfg(test)]
mod tests
{
    use super::*;
    
    #[test]
    fn std_in_out_err_fileno()
    {
	#[inline(always)]
	fn test_fileno<const EXPECTED: RawFd>(expected_name: &'static str, got: RawFd)
	{
	    assert_eq!(EXPECTED, got, "{expected_name} invalid: expected: {EXPECTED}, got {got}");
	}

	test_fileno::<STDIN_FILENO>("STDIN_FILENO", std::io::stdin().as_raw_fd());
	test_fileno::<STDOUT_FILENO>("STDOUT_FILENO", std::io::stdout().as_raw_fd());
	test_fileno::<STDERR_FILENO>("STDERR_FILENO", std::io::stderr().as_raw_fd());
    }

    #[test]
    fn test_readwrite()
    {
	let mut input = ManagedFD::from(memory::MemoryFile::new().unwrap());
	let mut output = memory::MemoryFile::new().unwrap();
	assert_eq!(std::io::copy(&mut input, &mut output).unwrap(), 0, "Bad read");

    }
}
