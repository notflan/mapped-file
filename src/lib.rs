
#[macro_use] extern crate lazy_static;

use libc::{
    mmap,
    MAP_FAILED,
};

use std::{
    os::unix::prelude::*,
    ops,
    mem,
    ptr::{
	self,
	NonNull,
    },
    io,
    fmt, error,
    
    borrow::{
	Borrow, BorrowMut,
    }
};

mod ffi;
use ffi::c_try;

pub mod hugetlb;
pub mod file;

use hugetlb::{
    HugePage,
    MapHugeFlag,
};

mod uniq;
use uniq::UniqueSlice;

mod flags;
pub use flags::*;

pub mod err;
use err::{
    os_error,
    opaque,
};


#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
struct MappedSlice(UniqueSlice<u8>);

impl ops::Drop for MappedSlice
{
    #[inline]
    fn drop(&mut self) 
    {
	unsafe {
            libc::munmap(self.0.as_mut_ptr() as *mut _, self.0.len());
	}
    }
}

/// A memory mapping over file `T`.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct MappedFile<T>
{
    file: T,
    map: MappedSlice,
}

impl<T: AsRawFd> MappedFile<T> {
    /// Map the file `file` to `len` bytes with memory protection as provided by `perm`, and mapping flags provided by `flags`.
    /// # Mapping flags
    /// The trait `MapFlags` is used to allow user-defined configurations of `mmap()`, but the `Flags` enum should usually be used for this, or `()`, which behaves the same as `Flags::default()`.
    ///
    /// # Returns
    /// If `mmap()` fails, then the current `errno` is returned alongside the `file` that was passed in, otherwise, a new mapping is
    /// constructed over `file`, and that is returned.
    ///
    /// # Panics
    /// If `mmap()` succeeds, but returns an invalid address (e.g. 0)
    pub fn try_new(file: T, len: usize, perm: Perm, flags: impl flags::MapFlags) -> Result<Self, TryNewError<T>>
    {
	#[inline(never)]
	#[cold]
        fn _panic_invalid_address() -> !
        {
            panic!("Invalid/unsupported address returned from mmap()")
        }
	const NULL: *mut libc::c_void = ptr::null_mut();
        let fd = file.as_raw_fd();
        let slice = match unsafe {
            mmap(ptr::null_mut(), len, perm.get_prot(), flags.get_mmap_flags(), fd, 0)
        } {
            MAP_FAILED => return Err(TryNewError::wrap_last_error(file)),
            NULL => _panic_invalid_address(),
            ptr => unsafe {
                UniqueSlice {
                    mem: NonNull::new_unchecked(ptr as *mut u8),
                    end: match NonNull::new((ptr as *mut u8).add(len)) {
			Some(n) => n,
			_ => _panic_invalid_address(),
		    },
                }
            },
        };
        Ok(Self {
            file,
            map: MappedSlice(slice)
        })
    }
    
    /// Map the file `file` to `len` bytes with memory protection as provided by `perm`, and
    /// mapping flags provided by `flags`.
    ///
    /// # Returns
    /// If `mmap()` fails, then the current `errno` set by `mmap()` is returned, otherwise, a new mapping is
    /// constructed over `file`, and that is returned.
    /// If `mmap()` fails, `file` is dropped. To retain `file`, use `try_new()`.
    ///
    /// # Panics
    /// If `mmap()` succeeds, but returns an invalid address (e.g. 0)
    #[inline] 
    pub fn new(file: T, len: usize, perm: Perm, flags: Flags) -> io::Result<Self>
    {
	Self::try_new(file, len, perm, flags).map_err(Into::into)
    }

    /// Sync the mapped memory to the backing file store via `msync()`.
    ///
    /// If this is a private mapping, or is mapped over a private file descriptor that does not refer to on-disk persistent storage, syncing the data is usually pointless.
    ///
    /// # Returns
    /// If `msync()` fails.
    pub fn flush(&mut self, flush: Flush) -> io::Result<()>
    {
        use libc::msync;
        match unsafe {
	    msync(self.map.0.as_mut_ptr() as *mut _, self.map.0.len(), flush.get_ms())
	} {
            0 => Ok(()),
            _ => Err(io::Error::last_os_error())
        }
    }

    /// Replace the mapped file object with another that aliases the same file descriptor.
    ///
    /// # Warning
    /// * The old file object is *not* dropped to prevent the file descriptor being closed. (see `replace_inner_unchecked()`).
    ///  If `T` contains other resources, this can cause a memory leak.
    ///
    /// # Panics
    /// If `other`'s `AsRawFd` impl *does not* alias the already contained `T`'s.
    pub fn replace_inner<U: AsRawFd>(self, other: U) -> MappedFile<U>
    {
        assert_eq!(self.file.as_raw_fd(), other.as_raw_fd(), "File descriptors must alias");
	unsafe {
	    let (this, file) = self.replace_inner_unchecked(other);
	    mem::forget(file);
	    this
	}
    }
    
    /// Unmap the memory contained in `T` and return it.
    /// Before the memory is unmapped, it is `msync()`'d according to `flush`.
    ///
    /// # Panics
    /// If `msync()` fails.
    #[inline]
    pub fn into_inner_synced(mut self, flush: Flush) -> T
    {
        self.flush(flush).expect("Failed to sync data");
        drop(self.map);
        self.file
    }
}

impl<T> MappedFile<T> {
    
    
    #[inline(always)]
    fn raw_parts(&self) -> (*mut u8, usize)
    {
        (self.map.0.mem.as_ptr(), self.map.0.len())
    } 

    /// Set advise according to `adv`, and optionally advise the kernel on if the memory will be needed or not.
    pub fn advise(&mut self, adv: Advice, needed: Option<bool>) -> io::Result<()>
    {
        use libc::{
            madvise,
            MADV_WILLNEED,
            MADV_DONTNEED
        };
        let (addr, len) = self.raw_parts();
        match unsafe { madvise(addr as *mut _, len, adv.get_madv() | needed.map(|n| n.then(|| MADV_WILLNEED).unwrap_or(MADV_DONTNEED)).unwrap_or(0)) } {
            0 => Ok(()),
            _ => Err(io::Error::last_os_error())
        }
    }

    /// With advice, used as a builder-pattern alternative for `advise()`.
    ///
    /// # Returns
    /// If `madvise()` fails, then the `io::Error` along with the previous instance is returned.
    #[inline(always)] 
    pub fn try_with_advice(mut self, adv: Advice, needed: Option<bool>) -> Result<Self, TryNewError<Self>>
    {
        match self.advise(adv, needed) {
            Ok(_) => Ok(self),
	    Err(error) => Err(TryNewError {
		error: Box::new(error),
		value: self,
	    })
	}
    }

    /// With advice, used as a builder-pattern alternative for `advise()`.
    ///
    /// # Returns
    /// If `madvise()` fails, then the mapping is dropped and the error is returned. To keep the previous instance if the call failes, use `try_with_advice()`.
    #[inline] 
    pub fn with_advice(self, adv: Advice, needed: Option<bool>) -> io::Result<Self>
    {
	self.try_with_advice(adv, needed).map_err(Into::into)
    }
    
    /// Replace the inner file with another without checking static or dynamic bounding.
    /// This function is extremely unsafe if the following conditions are not met in entirity.
    ///
    /// # Safety
    /// * `U` and `T` **must** have an `AsRawFd::as_raw_fd()` impl that returns the same `RawFd` value unconditionally.
    /// * The returned `T` in the tuple **must not** attempt close the file descriptor while the returned `MappedFile<U>` in the tuple is alive.
    /// * The returned values **should not** *both* attempt to close the file descriptor when dropped. To prevent the `MappedFile<U>` from attempting to close the file descriptor, use `MappedFile::into_inner()` and ensure `U` does not close the file descriptor while `T` is alive. Alternatively, use a mechanism of `T` to prevent it from closing the file descriptor while `U` is alive.
    #[inline(always)]
    pub unsafe fn replace_inner_unchecked<U>(self, other: U) -> (MappedFile<U>, T)
    {
        let MappedFile{ file, map } = self;
        (MappedFile {
            file: other,
            map
        }, file)
    }

    /// Unmap the memory contained in `T` and return it.
    ///
    /// # Warning
    /// If the map is shared, or refers to a persistent file on disk, you should call `flush()`
    /// first or use `into_inner_synced()`
    #[inline] 
    pub fn into_inner(self) -> T
    {
        drop(self.map);
        self.file
    }

    /// The size of the mapped memory
    #[inline]
    pub fn len(&self) -> usize
    {
        self.map.0.len()
    }

    /// Get a slice of the mapped memory
    #[inline]
    pub fn as_slice(&self) -> &[u8]
    {
        &self.map.0[..]
    }

    /// Get a mutable slice of the mapped memory
    #[inline]
    pub fn as_slice_mut(&mut self) -> &mut [u8]
    {
        &mut self.map.0[..]
    }

    /// Get a raw slice of the mapped memory
    #[inline] 
    pub fn as_raw_slice(&self) -> *const [u8]
    {
	self.map.0.as_raw_slice()
    }

    /// Get a raw mutable slice of the mapped memory
    #[inline]
    pub fn as_raw_slice_mut(&mut self) -> *mut [u8]
    {
	self.map.0.as_raw_slice_mut()
    }

    /// Checks if the mapping dangles (i.e. `len() == 0`.)
    #[inline]
    pub fn is_empty(&self) -> bool
    {
        self.map.0.is_empty()
    }
}

/// Error returned when mapping operation fails
pub struct TryNewError<T: ?Sized>
{
    error: Box<io::Error>,
    value: T
}

impl<T:?Sized> error::Error for TryNewError<T>
{
    #[inline] 
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
	Some(self.error.as_ref())
    }
}

impl<T:?Sized> fmt::Display for TryNewError<T>
{
    #[inline] 
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
	write!(f, "error in mapping of type {}", std::any::type_name::<T>())
    }
}

impl<T:?Sized> fmt::Debug for TryNewError<T>
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
	f.debug_struct("TryNewError")
	    .field("error", &self.error)
	    .finish_non_exhaustive()
    }
}

impl<T: ?Sized> TryNewError<T>
{
    /// A reference to the value
    #[inline] 
    pub fn value(&self) -> &T
    {
	&self.value
    }
    /// A mutable reference to the value
    #[inline] 
    pub fn value_mut(&mut self) -> &mut T
    {
	&mut self.value
    }
    /// A reference to the IO error
    #[inline] 
    pub fn error(&self) -> &io::Error
    {
	&self.error
    }
    /// Consume a boxed instance and return the boxed IO error.
    #[inline] 
    pub fn into_error_box(self: Box<Self>) -> Box<io::Error>
    {
	self.error
    }
}

impl<T> TryNewError<T>
{
    #[inline] 
    fn wrap_last_error(value: T) -> Self
    {
	Self {
	    error: Box::new(io::Error::last_os_error()),
	    value,
	}
    }
    /// Consume into the contained value 
    #[inline] 
    pub fn into_inner(self) -> T
    {
	self.value
    }

    /// Consume into the IO error
    #[inline] 
    pub fn into_error(self) -> io::Error
    {
	*self.error
    }
    /// Consume into the value and the error.
    #[inline] 
    pub fn into_parts(self) -> (T, io::Error)
    {
	(self.value, *self.error)
    }
}

impl<T: ?Sized> From<Box<TryNewError<T>>> for io::Error
{
    #[inline] 
    fn from(from: Box<TryNewError<T>>) -> Self
    {
	*from.error
    }
}

impl<T> From<TryNewError<T>> for io::Error
{
    #[inline] 
    fn from(from: TryNewError<T>) -> Self
    {
	from.into_error()
    }
}

impl<T: AsRawFd> Borrow<T> for MappedFile<T>
{
    #[inline]
    fn borrow(&self) -> &T
    {
        &self.file
    }
}

impl<T> Borrow<[u8]> for MappedFile<T>
{
    #[inline]
    fn borrow(&self) -> &[u8]
    {
        self.as_slice()
    }
}

impl<T> BorrowMut<[u8]> for MappedFile<T>
{
    #[inline]
    fn borrow_mut(&mut self) -> &mut [u8]
    {
        self.as_slice_mut()
    }
}

impl<T> ops::Deref for MappedFile<T>
{
    type Target=  [u8];
    #[inline]
    fn deref(&self) -> &Self::Target
    {
        self.as_slice()
    }
}
impl<T> ops::DerefMut for MappedFile<T>
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        self.as_slice_mut()
    }
}

/// Used for anonymous mappings with `MappedFile`.
///
/// # Safety
/// The `AsRawFd` impl of this structure always returns `-1`. It should only be used with `MappedFile`, as this is an invlalid file descriptor in all other contexts.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy)]
pub struct Anonymous;

impl AsRawFd for Anonymous
{
    #[inline(always)] 
    fn as_raw_fd(&self) -> RawFd {
	-1
    }
}

//TODO: Continue copying from `utf8encode` at the //TODO (cont.) line

