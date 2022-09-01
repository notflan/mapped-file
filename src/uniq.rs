//! Unique owned slices of virtual memory
use std::{
    ptr::{
	self,
	NonNull,
    },
    mem,
    borrow::{
	Borrow, BorrowMut
    },
    ops,
    hash::Hash,
};

/// A slice in which nothing is aliased. The `UniqueSlice<T>` *owns* all memory in between `mem` and `end`.
#[derive(Debug)]
pub struct UniqueSlice<T> {
    pub(crate) mem: NonNull<T>, 
    pub(crate) end: NonNull<T>,
}

impl<T> ops::Drop for UniqueSlice<T> {
#[inline]
    fn drop(&mut self) {
        if mem::needs_drop::<T>() {
            unsafe {	
		ptr::drop_in_place(self.as_raw_slice_mut());
	    }
        }
    }
}

impl<T> Borrow<[T]> for UniqueSlice<T>
{
#[inline]
    fn borrow(&self) -> &[T]
    {
        self.as_slice()
    }
}
impl<T> BorrowMut<[T]> for UniqueSlice<T>
{
#[inline]
    fn borrow_mut(&mut self) -> &mut [T]
    {
        self.as_slice_mut()
    }
}

impl<T> Hash for UniqueSlice<T>
{
#[inline]
    fn hash<H>(&self, hasher: &mut H)
        where H: std::hash::Hasher
    {
        ptr::hash(self.mem.as_ptr() as *const _, hasher);
        ptr::hash(self.end.as_ptr() as *const _, hasher);
    }
}

impl<T> Ord for UniqueSlice<T>
{
#[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering
    {
        self.end.cmp(&other.end)
    }
}
impl<T> PartialOrd for UniqueSlice<T>
{
#[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering>
    {
        self.end.partial_cmp(&other.end)
    }
}

impl<T> Eq for UniqueSlice<T>{}
impl<T> PartialEq for UniqueSlice<T>
{
#[inline]
    fn eq(&self, other: &Self) -> bool
    {
	ptr::eq(self.mem.as_ptr(), other.mem.as_ptr()) &&
        ptr::eq(self.end.as_ptr(), other.end.as_ptr())
    }
}

impl<T> AsRef<[T]> for UniqueSlice<T>
{
#[inline]
    fn as_ref(&self) -> &[T]
    {
        self.as_slice()
    }
}

impl<T> AsMut<[T]> for UniqueSlice<T>
{
#[inline]
    fn as_mut(&mut self) -> &mut [T]
    {
        self.as_slice_mut()
    }
}

impl<T> ops::Deref for UniqueSlice<T>
{
    type Target= [T];
#[inline]
    fn deref(&self) -> &Self::Target
    {
        self.as_slice()
    }
}

impl<T> ops::DerefMut for UniqueSlice<T>
{
#[inline]
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        self.as_slice_mut()
    }
}

impl<T> UniqueSlice<T>
{
#[inline(always)]
    pub fn is_empty(&self) -> bool
    {
        ptr::eq(self.mem.as_ptr(), self.end.as_ptr())
    }
#[inline(always)]
    pub fn get_ptr(&self) -> Option<NonNull<T>>
    {
        if self.is_empty() {
            None
        } else {
            Some(self.mem)
        }
    }
}

impl<T> UniqueSlice<T>
{
#[inline]
    pub fn as_slice(&self) -> &[T]
    {
        unsafe { &*self.as_raw_slice() }
    }
#[inline]
    pub fn as_slice_mut(&mut self) -> &mut [T]
    {
        unsafe { &mut *self.as_raw_slice_mut() }
    }
#[inline(always)]
    pub fn as_raw_slice_mut(&mut self) -> *mut [T]
    {
        if self.is_empty() {
            ptr::slice_from_raw_parts_mut(self.mem.as_ptr(), 0)
        } else {
            ptr::slice_from_raw_parts_mut(self.mem.as_ptr(), self.len())
        }
    }
#[inline(always)]
    pub fn as_raw_slice(&self) -> *const [T]
    {
        if self.is_empty() {
            ptr::slice_from_raw_parts(self.mem.as_ptr() as *const _, 0)
        } else {
            ptr::slice_from_raw_parts(self.mem.as_ptr() as *const _, self.len())
        }
    }
#[inline]
    pub fn len(&self) -> usize
    {
	unsafe {
            (self.end.as_ptr().sub(self.mem.as_ptr() as usize) as usize) / mem::size_of::<T>()
	}
    }
#[inline(always)]
    pub fn first(&self) -> Option<&T>
    {
        if self.is_empty() {
            None
        } else {
            Some(unsafe { &*(self.mem.as_ptr() as *const _) })
        }
    }
#[inline(always)]
    pub fn last(&self) -> Option<&T>
    {
        if self.is_empty() {
            None
        } else {
            Some(unsafe { &*(self.end.as_ptr().sub(1) as *const _) })
        }
    }
#[inline(always)]
    pub fn first_mut(&mut self) -> Option<&mut T>
    {
        if self.is_empty() {
            None
        } else {
            Some(unsafe { &mut *self.mem.as_ptr() })
        }
    }
#[inline(always)]
    pub fn last_mut(&mut self) -> Option<&mut T>
    {
        if self.is_empty() {
            None
        } else {
            Some(unsafe { &mut *self.end.as_ptr().sub(1) })
        }
    }

    /// Get a range of pointers in the format `mem..end`. Where `mem` is the first element and `end` is 1 element past the last element.
    #[inline]
    pub const fn as_ptr_range(&self) -> std::ops::Range<*mut T>
    {
	self.mem.as_ptr()..self.end.as_ptr()
    }
}

