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
