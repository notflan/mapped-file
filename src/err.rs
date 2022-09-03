//! Wrapping `errno` error types
use super::*;
use std::ffi::c_int;
use std::{
    fmt, error
};

/// Construct an ad-hoc error wrapping the last OS error.
macro_rules! os_error {
    ($fmt:literal $(, $args:expr)*) => {
	{
	    #[derive(Debug)]
            struct AdHoc<'a>(::std::fmt::Arguments<'a>);
	    impl<'a> ::std::fmt::Display for AdHoc<'a>
	    {
		#[inline] 
		fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
		{
		    write!(f, "{}", self.0)
		}
	    }
	    $crate::err::WrappedOSError::last_os_error(AdHoc(::std::format_args!($fmt $(, $args)*)))
	}
    };
    ($(#[$outer:meta])* $vis:vis struct $name:ident => $fmt:literal $(; $($rest:tt)*)?) => {

	$(#[$outer])*
	    #[derive(Debug)]
	#[repr(transparent)]
	$vis struct $name($crate::err::WrappedOSError<&'static str>);

	impl ::std::fmt::Display for $name
	{
	    #[inline] 
	    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> fmt::Result
	    {
		f.write_str($fmt)
	    }
	}
	impl ::std::error::Error for $name {
	    #[inline] 
	    fn source(&self) -> Option<&(dyn ::std::error::Error + 'static)>
	    {
		self.0.source()
	    }
	}

	#[allow(unused)]
	impl $name {
	    #[inline(always)] 
	    fn new() -> Self
	    {
		Self($crate::err::WrappedOSError::last_os_error($fmt))
	    }
	    #[inline(always)] 
	    fn into_inner(self) -> $crate::err::WrappedOSError<&'static str>
	    {
		self.0
	    }
	}
	impl ::std::ops::Deref for $name {
	    type Target = $crate::err::WrappedOSError<&'static str>;
	    #[inline] 
	    fn deref(&self) -> &Self::Target
	    {
		&self.0
	    }
	}
	impl ::std::ops::DerefMut for $name {
	    #[inline] 
	    fn deref_mut(&mut self) -> &mut Self::Target
	    {
		&mut self.0
	    }
	}

	$(
	    $crate::os_error! {
		$($rest)*
	    }
	)?
    };
    () => {};
}
pub(crate) use os_error;

const _: () = {
    os_error!(struct Test => "Test error");
    const fn t<E: ?Sized + error::Error>() {}
    fn r<E: ?Sized + error::Error>(_: &E) {}
    fn test() {
	r(&os_error!("Some error message"));
    }
    t::<Test>()
};

/// Wraps a piece of context over an OS error
pub struct WrappedOSError<E: ?Sized>(io::Error, E);

impl<E: fmt::Debug> WrappedOSError<E>
{
    pub(crate) fn last_os_error(ctx: E) -> Self
    {
	Self(io::Error::last_os_error(), ctx)
    }

    pub(crate) fn from_os_error(raw: c_int, ctx: E) -> Self
    {
	Self(io::Error::from_raw_os_error(raw), ctx)
    }
}

impl<E: ?Sized> WrappedOSError<E>
{
    #[inline] 
    pub fn error(&self) -> &io::Error
    {
	&self.0
    }
    #[inline] 
    pub fn raw_error(&self) -> c_int
    {
	self.0.raw_os_error().unwrap()
    }
    #[inline] 
    pub fn context(&self) -> &E
    {
	&self.1
    }
}

impl<E> From<WrappedOSError<E>> for io::Error
{
    #[inline] 
    fn from(from: WrappedOSError<E>) -> Self
    {
	from.0
    }
}


impl<E: ?Sized> error::Error for WrappedOSError<E>
where WrappedOSError<E>: fmt::Display + fmt::Debug
{
    #[inline] 
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
	Some(&self.0)
    }
}

impl<E: ?Sized> fmt::Display for WrappedOSError<E>
where E: fmt::Debug
{
    #[inline] 
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
	write!(f, "{:?}", &self.1)
    }
}
impl<E: ?Sized> fmt::Debug for WrappedOSError<E>
where E: fmt::Display
{
    #[inline] 
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
	write!(f, "{}", &self.1)
    }
}


