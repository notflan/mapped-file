//! Provides a wrapper over `RawFd` that does not close it on drop.
//! This can be useful for aliasing file descriptors.
use super::*;

/// Represents a `RawFd` but does not provide any ownership of it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct UnmanagedFD(NonNegativeI32);

impl UnmanagedFD {
    #[inline] 
    pub fn new(alias: &(impl AsRawFd + ?Sized)) -> Self
    {
	Self(alias.as_raw_fd().into())
    }

    #[inline] 
    pub(super) const fn new_or_panic(raw: RawFd) -> Self
    {
	Self(NonNegativeI32::new_or_panic(raw))
    }

    #[inline]
    pub const unsafe fn new_unchecked(raw: RawFd) -> Self
    {
	Self(NonNegativeI32::new_unchecked(raw))
    }

    #[inline] 
    pub const fn get(&self) -> RawFd
    {
	self.0.get()
    }
}

impl From<RawFd> for UnmanagedFD
{
    #[inline] 
    fn from(from: RawFd) -> Self
    {
	debug_assert!(from >= 0, "Invalid file descriptor");
	Self(from.into())
    }
}

impl From<UnmanagedFD> for RawFd
{
    #[inline] 
    fn from(from: UnmanagedFD) -> Self
    {
	from.get()
    }
}

// No impl for `IntoRawFd` because `UnmanagedFD` is not owning

impl FromRawFd for UnmanagedFD
{
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
	Self(fd.into())
    }
}


impl AsRawFd for UnmanagedFD
{
    #[inline(always)] 
    fn as_raw_fd(&self) -> RawFd {
	self.0.get()
    }
}

//TODO: implement a full version of the temporary struct `UnmanagedFD` from `utf8encode`
