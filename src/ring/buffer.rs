//! Traits and types used for mapping a R^W send-recv buffer `(tx, rx)`
//!
//! See `MappedFile::try_new_buffer()`
use super::*;
use std::{
    borrow::Borrow,
    ops,
    sync,
    rc,
};

pub trait TwoBufferProvider<T: ?Sized>
{
    type ControlWrapper: Borrow<T>;

    fn as_wrapper(&self) -> &Self::ControlWrapper;
    fn from_wrapper_boxed(r: &Self::ControlWrapper) -> Box<Self>;

    #[inline(always)] 
    fn from_wrapper(r: &Self::ControlWrapper) -> Self
    where Self: Sized {
	*Self::from_wrapper_boxed(r)
    }

    #[inline(always)] 
    fn inner(&self) -> &T
    {
	self.as_wrapper().borrow()
    }

    fn from_boxed(value: Box<T>) -> Box<Self>;

    #[inline(always)] 
    fn from_value(value: T) -> Self
    where T: Sized,
	  Self: Sized
    {
	*Self::from_boxed(Box::new(value))
    }
}

/// For thread-sharable buffer holds
#[derive(Debug)]
pub struct Shared<T: ?Sized>(sync::Arc<T>);

/// For non thread-sharable buffer holds
#[derive(Debug)]
pub struct Private<T: ?Sized>(rc::Rc<T>);

impl<T: ?Sized> TwoBufferProvider<T> for Shared<T> {
    type ControlWrapper = sync::Arc<T>;

    #[inline(always)] 
    fn as_wrapper(&self) -> &Self::ControlWrapper {
	&self.0
    }

    #[inline] 
    fn from_boxed(value: Box<T>) ->Box<Self> {
	Box::new(Self(From::from(value)))
    }

    #[inline(always)]
    fn from_value(value: T) -> Self
    where T: Sized,
	  Self: Sized {
	Self(sync::Arc::new(value))
    }

    #[inline] 
    fn from_wrapper_boxed(r: &Self::ControlWrapper) -> Box<Self> {
	Box::new(Self(r.clone()))
    }
    #[inline(always)] 
    fn from_wrapper(r: &Self::ControlWrapper) -> Self
    where Self: Sized {
	Self(r.clone())
    }
}

impl<T: ?Sized + AsRawFd> AsRawFd for Shared<T>
{
    #[inline(always)] 
    fn as_raw_fd(&self) -> RawFd {
	self.as_wrapper().as_raw_fd()
    }
}

impl<T: ?Sized> TwoBufferProvider<T> for Private<T> {
    type ControlWrapper = rc::Rc<T>;

    #[inline(always)] 
    fn as_wrapper(&self) -> &Self::ControlWrapper {
	&self.0
    }
    
    #[inline] 
    fn from_boxed(value: Box<T>) ->Box<Self> {
	Box::new(Self(From::from(value)))
    }
    
    #[inline(always)]
    fn from_value(value: T) -> Self
    where T: Sized,
	  Self: Sized {
	Self(rc::Rc::new(value))
    }
    
    #[inline] 
    fn from_wrapper_boxed(r: &Self::ControlWrapper) -> Box<Self> {
	Box::new(Self(r.clone()))
    }
    #[inline(always)] 
    fn from_wrapper(r: &Self::ControlWrapper) -> Self
    where Self: Sized {
	Self(r.clone())
    }
}

impl<T: ?Sized + AsRawFd> AsRawFd for Private<T>
{
    #[inline(always)] 
    fn as_raw_fd(&self) -> RawFd {
	self.as_wrapper().as_raw_fd()
    }
}

impl<T: ?Sized> Shared<T>
{
    /// Check if the connected mapping has not been dropped.
    #[inline] 
    pub fn is_connected(&self) -> bool
    {
	sync::Arc::strong_count(&self.0) > 1
    }

    /// Consume into an `Arc` instance over the file handle.
    #[inline] 
    pub fn into_arc(self) -> sync::Arc<T>
    {
	self.0
    }

    /// Get a reference of the file handle.
    #[inline] 
    pub fn inner(&self) -> &T
    {
	&self.0
    }
}
impl<T: ?Sized> Private<T>
{
    /// Check if the connected mapping has not been dropped.
    #[inline] 
    pub fn is_connected(&self) -> bool
    {
	rc::Rc::strong_count(&self.0) > 1
    }

    /// Consume into an `Rc` instance over the file handle.
    #[inline] 
    pub fn into_rc(self) -> rc::Rc<T>
    {
	self.0
    }

    /// Get a reference of the file handle.
    #[inline] 
    pub fn inner(&self) -> &T
    {
	&self.0
    }
}

//TODO: use `dup()` to turn (MappedFile<B>, MappedFile<B>) -> (MappedFile<impl FromRawFd>, MappedFile<impl FromRawFd>)

pub trait BufferExt<T>
{
    fn detach(txrx: Self) -> (MappedFile<T>, MappedFile<T>);
}

impl<B, T> BufferExt<T> for (MappedFile<B>, MappedFile<B>)
where B: TwoBufferProvider<T> + AsRawFd,
T: FromRawFd,
{
    /// Detach a mapped dual buffer 2-tuple into regular mapped inner types.
    #[inline] 
    fn detach((itx, irx): Self) -> (MappedFile<T>, MappedFile<T>) {
	#[cold]
	#[inline(never)]
	fn _panic_bad_dup(fd: RawFd) -> !
	{
	    panic!("Failed to dup({fd}): {}", io::Error::last_os_error())
	}
	let tx = itx.file.as_raw_fd();
	let rx = irx.file.as_raw_fd();
	
	let (f0, f1) = unsafe {
	    let fd1 = libc::dup(tx);
	    if fd1 < 0 {
		_panic_bad_dup(tx);
	    }
	    let fd2 = libc::dup(rx);
	    if fd2 < 0 {
		_panic_bad_dup(rx);
	    }
	    (T::from_raw_fd(fd1), T::from_raw_fd(fd2))
	};
	(MappedFile {
	    map: itx.map,
	    file: f0,
	}, MappedFile {
	    map: irx.map,
	    file: f1
	})
    }
}
