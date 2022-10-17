//! Extensions
use super::*;
use std::{
    ops,
    borrow::{
	Borrow, BorrowMut
    },
    mem::{
	ManuallyDrop,
	MaybeUninit,
    },
};

/// Defer an expression call
macro_rules! defer {
    (move => $expr:expr) => {
	$crate::ext::Deferred(move || {
	    $expr
	})
    };
    (ref => $expr:expr) => {
        $crate::ext::Deferred(|| {
	    $expr
	})
    };
    (move $value:expr => $expr:expr)  => {
	$crate::ext::DeferredDrop($value, move |a| {
	    $expr(a)
	})
    };
    (ref $value:expr => $expr:expr)  => {
	$crate::ext::DeferredDrop($value, |a| {
	    $expr(a)
	})
    };
}
pub(crate) use defer;

/// Defer calling `F` until the destructor is ran
#[repr(transparent)]
pub struct Deferred<F: ?Sized +  FnOnce() -> ()>(ManuallyDrop<F>);

/// Defer dropping this value until the container is dropped. The function `F` will be called on the value at drop time.
pub struct DeferredDrop<T, F: ?Sized + FnOnce(T) -> ()>(T, ManuallyDrop<F>);

impl<F: ?Sized+ (FnOnce() -> ())> ops::Drop for Deferred<F>
{
    #[inline] 
    fn drop(&mut self) {
	todo!("How tf do we get a Box<F> from ManuallyDrop<F>??? We might just have to `transmute()` its `assume_init_mut()` to a higher lifetime");
	/*let func_ptr = unsafe {
	    ptr::read(MaybeUninit::assume_init_mut(&mut self.0) as *mut F;
	    // aosidaosij oijoi jaoile FUCK THIS SHIT, I JUST WANNA CALL THE DAMN THINGt p: *mut F =  ManuallyDrop::borrow_mut(&mut self.0) as *mut _;
	};
	func_ptr();*/
    }
}

impl<T, F: ?Sized+ FnOnce(T) -> ()> ops::Drop for DeferredDrop<T, F>
{
    #[inline] 
    fn drop(&mut self) {
	todo!("See above...")
	//self.1(self.0);
    }
}

impl<T, F: ?Sized + FnOnce(T) -> ()> ops::DerefMut for DeferredDrop<T,F>
{
    #[inline] 
    fn deref_mut(&mut self) -> &mut Self::Target {
	&mut self.0
    }
}
impl<T, F: ?Sized + FnOnce(T) -> ()> ops::Deref for DeferredDrop<T,F>
{
    type Target = T;
    #[inline] 
    fn deref(&self) -> &Self::Target {
	&self.0
    }
}

impl<T, F: ?Sized + FnOnce(T) -> ()> Borrow<T> for DeferredDrop<T,F>
{
    #[inline(always)] 
    fn borrow(&self) -> &T {
	&self.0
    }
}

impl<T, F: ?Sized + FnOnce(T) -> ()> BorrowMut<T> for DeferredDrop<T,F>
{
    #[inline(always)] 
    fn borrow_mut(&mut self) -> &mut T {
	&mut self.0
    }
}
