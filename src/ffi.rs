//! Useful for C-interop
use super::*;

macro_rules! c_try {
    ($call:expr => $invalid:literal; $fmt:literal $(, $args:expr)*) => {
	{
            #[inline(never)]
            #[cold]
            fn _panic_bad_c_call<'a>(args: ::std::fmt::Arguments<'a>) -> !
            {
		panic!("C call failed (invalid return {}): {}", $invalid, args)
            }

	    let res = unsafe { $call };
	    if res == $invalid {
		_panic_bad_c_call(format_args!($fmt $(, $args)*))
	    }
	    res
	}
    };
    ($call:expr => if $func:expr; $fmt:literal $(, $args:expr)*) => {
	{
	    #[inline(never)]
            #[cold]
            fn _panic_bad_c_call<'a, T: ::std::fmt::Display>(invalid: T, args: ::std::fmt::Arguments<'a>) -> !
            {
		panic!("C call failed (invalid return {}): {}", invalid, args)
            }
	    let res = unsafe { $call };
	    if $func(res) {
		_panic_bad_c_call(res, format_args!($fmt $(, $args)*));
	    }
	    res
	}
    };
    (? $call:expr => if $func:expr; $fmt:literal $(, $args:expr)*) => {
	{
	    let res = unsafe { $call };
	    if $func(res) {
		Err(FFIError::from_last_error(res, format_args!($fmt $(, $args)*)))
	    } else {
		Ok(res)
	    }
	}
    };
    (? $call:expr => $invalid:literal; $fmt:literal $(, $args:expr)*) => {
	{
	    let res = unsafe { $call };
	    if res == $invalid {
		Err(FFIError::from_last_error($invalid, format_args!($fmt $(, $args)*)))
	    } else {
		Ok(res)
	    }
	}
    };
    /* Eh... Idk why this doesn't work...
    ($call:expr => {$($invalid:pat $(if $pred:pat)?),+} => $fmt:literal $(, $args:expr)*) => {
    {
    #[inline(never)]
    #[cold]
    fn _panic_bad_c_call<'a, T: ::std::fmt::Display>(invalid: T, args: ::std::fmt::Arguments<'a>) -> !
    {
    panic!("C call failed (invalid return {}): {}", invalid, args)
}
    let res = $call;
    match res {
    $($invalid $(if $pred)? => _panic_bad_c_call(res, format_args!($fmt $(, $args)*))),*
    x => x,
}
}
};*/
}
pub(crate) use c_try;

/// Error context for a failed C call.
/// Returns the invalid return value, the `errno` error, and a message.
#[derive(Debug)]
pub struct FFIError<'a, T>(T, io::Error, fmt::Arguments<'a>);

impl<'a, T> FFIError<'a, T>
where FFIError<'a, T>: error::Error
{
    #[inline(never)]
    #[cold]
    fn from_last_error(value: T, arguments: fmt::Arguments<'a>) -> Self
    {
	Self(value, io::Error::last_os_error(), arguments)
    }   
}
    

impl<'a, T> AsRef<io::Error> for FFIError<'a, T>
{
    #[inline]
    fn as_ref(&self) -> &io::Error {
	&self.1
    }
}

impl<'a, T> FFIError<'a, T>
{
    /// A reference to the value
    #[inline] 
    pub fn value(&self) -> &T
    {
	&self.0
    }

    /// Clone an instance of the value
    #[inline] 
    pub fn to_value(&self) -> T
    where T: Clone
    {
	self.0.clone()
    }

    /// Consume into the value
    #[inline] 
    pub fn into_value(self) -> T
    {
	self.0
    }


    /// Consume into a recursive 2-tuple of `((value, error), message)`.
    #[inline] 
    pub fn into_parts(self) -> ((T, io::Error), impl fmt::Display + fmt::Debug + 'a)
    {
	((self.0, self.1), self.2)
    }

    /// A reference to the inner OS error
    #[inline] 
    pub fn error(&self) -> &io::Error
    {
	&self.1
    }

    /// Get a reference to an opaque type that can be formatted into the message
    #[inline] 
    pub fn message(&self) -> &(impl fmt::Display + fmt::Debug + 'a)
    {
	&self.2
    }

    /// Consume an opaque type that can be formatted into the message
    pub fn into_message(self) -> impl fmt::Display + fmt::Debug + 'a
    {
	self.2
    }
/* This doesn't work...
    /// Render any referenced arguments in the message into a string, reducing the lifetime requirement of the message to `'static`.
    ///
    /// # Notes
    /// If `T` is not also `'static`, then the resulting instance will not be `'static` itself. If `T` is not `'static`, use `into_owned()` instead.
    #[inline] 
    pub fn message_into_owned(self) -> FFIError<'static, T>
    {
	FFIError(self.0, self.1, format_args!("{}", self.2.to_string()))
    }

    /// Clone any referenced arguments of the message and the value into a non-referential object, reducing the lifetime requirements of the returned instance to `'static`.
    #[inline] 
    pub fn into_owned(self) -> FFIError<'static, T::Owned>
    where T: ToOwned,
    T::Owned: 'static
    {
	FFIError(self.0.to_owned(), self.1, format_args!("{}", self.2.to_string()))
}
    */
}

impl<'a, T> error::Error for FFIError<'a, T>
where FFIError<'a, T>: fmt::Display + fmt::Debug
{
    #[inline] 
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
	Some(&self.1)
    }
}
impl<'a, T: fmt::Debug> fmt::Display for FFIError<'a, T>
{
    #[inline] 
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
	write!(f, "C call failed (invalid return {:?}): {}", self.0, &self.2)
    }
}

impl<'a, T> From<FFIError<'a, T>> for io::Error
{
    #[inline] 
    fn from(from: FFIError<'a, T>) -> Self
    {
	from.1
    }
}

