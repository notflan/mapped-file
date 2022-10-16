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

    #[inline(always)] 
    fn inner(&self) -> &T
    {
	self.as_wrapper().borrow()
    }

    fn from_boxed(value: Box<T>) -> Box<Self>;
//TODO: How do we give enough info to caller to create this?	
}

/// For thread-sharable buffer holds
#[derive(Debug, Clone)]
pub struct Shared<T: ?Sized>(sync::Arc<T>);

/// For non thread-sharable buffer holds
#[derive(Debug, Clone)]
pub struct Private<T: ?Sized>(rc::Rc<T>);

impl<T: ?Sized> TwoBufferProvider<T> for Shared<T> {
    type ControlWrapper = sync::Arc<T>;

    #[inline(always)] 
    fn as_wrapper(&self) -> &Self::ControlWrapper {
	&self.0
    }

    #[inline] 
    fn from_boxed(value: Box<T>) ->Box<Self> {
	Box::new(Self(value.into()))
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
	Box::new(Self(value.into()))
    }
}

impl<T: ?Sized + AsRawFd> AsRawFd for Private<T>
{
    #[inline(always)] 
    fn as_raw_fd(&self) -> RawFd {
	self.as_wrapper().as_raw_fd()
    }
}
