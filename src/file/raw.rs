//! Internals for `RawFd`
use super::*;
use std::num::NonZeroU32;

/// Used for the base of valid file descriptors for non-null optimisation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy, PartialOrd, Ord)]
#[repr(transparent)]
pub(super) struct NonNegativeI32(NonZeroU32);

const RANGE_MASK: u32 = u32::MAX >> 1;
const NEG_FLAG: u32 = !RANGE_MASK;

#[inline(always)] 
const fn is_negative(val: i32) -> bool
{
    ((val as u32) & NEG_FLAG) != 0
}

impl NonNegativeI32
{
    #[inline] 
    pub fn new_or_else<F: FnOnce() -> Self>(raw: i32, f: F) -> Self
    {
	Self::new(raw).unwrap_or_else(f)
    }
    #[inline] 
    pub fn new_or(raw: i32, or: Self) -> Self
    {
	Self::new(raw).unwrap_or(or)
    }
    #[inline]
    pub const fn new_or_panic(raw: i32) -> Self
    {
	#[inline(never)]
	#[cold]
	const fn _panic_negative() -> !
	{
	    panic!("Negative integer passed to asserting panic")
	}
	match Self::new(raw) {
	    Some(v) => v,
	    None => _panic_negative()
	}
    }
    #[inline] 
    pub const fn new(raw: i32) -> Option<Self>
    {
	if is_negative(raw) {
	    None
	} else {
	    Some(Self(unsafe {NonZeroU32::new_unchecked(raw as u32 | NEG_FLAG)}))
	}
    }

    #[inline] 
    pub const fn get(self) -> i32
    {
	(self.0.get() & RANGE_MASK) as i32
    }

    #[inline] 
    pub const unsafe fn new_unchecked(raw: i32) -> Self
    {
	Self(NonZeroU32::new_unchecked(raw as u32 | NEG_FLAG))
    }
}

impl PartialOrd<i32> for NonNegativeI32
{
    #[inline] 
    fn partial_cmp(&self, other: &i32) -> Option<std::cmp::Ordering> {
	self.get().partial_cmp(other)
    }
}
impl PartialEq<i32> for NonNegativeI32
{
    #[inline] 
    fn eq(&self, other: &i32) -> bool
    {
	self.get().eq(other)
    }
}

impl PartialEq<NonNegativeI32> for i32
{
    #[inline] 
    fn eq(&self, other: &NonNegativeI32) -> bool
    {
	self.eq(&other.get())
    }
}

impl PartialOrd<NonNegativeI32> for i32
{
    #[inline] 
    fn partial_cmp(&self, other: &NonNegativeI32) -> Option<std::cmp::Ordering> {
	self.partial_cmp(&other.get())
    }
}

impl From<NonNegativeI32> for i32
{
    #[inline] 
    fn from(from: NonNegativeI32) -> Self
    {
	from.get()
    }
}

impl From<i32> for NonNegativeI32
{
    // Convenience `From` impl (panicking) 
    #[inline(always)] 
    fn from(from: i32) -> Self
    {
	Self::new_or_panic(from)
    }
}

/// Implements `io::Read` and `io::Write` for a type that implements an accessor for a raw file-descriptor.
///
/// Usage:
/// ```no_compile
/// struct HasFd(UnmanagedFD);
/// impl_io_for_fd(HasFd => .0);
/// ```
macro_rules! impl_io_for_fd {
    ($type:ty => .$($fd_path:tt)+) => {
	const _:() = {
	    use std::io;
	    #[inline(always)]
	    fn check_error() -> bool
	    {
		use libc::{
		    EINTR,
		    __errno_location,
		};
		match unsafe { *__errno_location() } {
		    EINTR => true,
		    _ => false,
		}
	    }
	    
	    impl io::Write for $type
	    {
		fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
		    match unsafe { libc::write(self.$($fd_path)+, buf.as_ptr()  as *const _, buf.len()) } {
			-1 => Err(io::Error::last_os_error()),
			n => Ok(n as usize)
		    }
		}
		fn write_all(&mut self, mut buf: &[u8]) -> io::Result<()> {
		    
		    loop {
			buf = match buf {
			    [] => break Ok(()),
			    buf => {
				match unsafe{ libc::write(self.$($fd_path)+, buf.as_ptr() as *const _, buf.len()) } {
				    -1 if check_error() => {
					return Err(io::Error::last_os_error());
				    },
				    -1 => continue,
				    0 => return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "write returned 0")),
				    n => &buf[(n as usize)..],
				}
			    },
			};
		    }
		}
		
		#[inline] 
		fn flush(&mut self) -> io::Result<()> {
		    Ok(())
		}
	    }

	    impl io::Read for $type
	    {
		fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>
		{
		    match unsafe { libc::read(self.$($fd_path)+, buf.as_mut_ptr() as *mut _, buf.len()) } {
			-1 => Err(io::Error::last_os_error()),
			n => Ok(n as usize),
		    }
		}
		fn read_exact(&mut self, mut buf: &mut [u8]) -> io::Result<()>
		{
		    loop {
			let n = match &mut buf {
			    [] => break Ok(()),
			    buf => {
				match unsafe { libc::read(self.$($fd_path)+, (**buf).as_mut_ptr() as *mut libc::c_void, buf.len()) } {
				    -1 if check_error() => {
					return Err(io::Error::last_os_error());
				    },
				    -1 => continue,
				    0 => return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "read returned 0")),
				    n => n as usize,
				}
			    }
			};
			buf = &mut buf[n..];
		    }
		}
	    }
	};
    };
}
pub(super) use impl_io_for_fd;
