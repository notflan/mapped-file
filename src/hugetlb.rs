//! Huge-page interface for `MappedFile<T>` and `MemoryFile`.
use super::*;
use std::{
    mem,
    hash,
    num::NonZeroUsize,
};
use libc::{
    c_int,
    MAP_HUGE_SHIFT,
};

/// Location in which the kernel exposes available huge-page sizes.
pub const HUGEPAGE_LOCATION: &'static str = "/sys/kernel/mm/hugepages/";

/// Represents a statically defined `MAP_HUGE_*` flag.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Copy)]
#[repr(transparent)]
pub struct MapHugeFlag(c_int);

impl Default for MapHugeFlag
{
    #[inline] 
    fn default() -> Self {
	Self(MAP_HUGE_SHIFT)
    }
}
#[inline(always)]
//TODO: XXX: Check this implementation of `log2<usize>()`... It seems slightly wrong...
const fn log2(n: usize) -> usize
{
    /*const fn num_bits<T>() -> usize {
    mem::size_of::<T>() * (u8::BITS as usize)
}*/
    usize::BITS as usize -  n.leading_zeros() as usize - 1
}

impl MapHugeFlag
{
    /// Create from a raw `MAP_HUGE_*` flag.
    ///
    /// # Safety
    /// The passed `flag` **must** be a valid bitmask representing a `MAP_HUGE_*` value **only**.
    #[inline] 
    pub const unsafe fn from_mask_unchecked(flag: c_int) -> Self
    {
	Self(flag)
    }

    /// The kernel's default huge-page size.
    pub const HUGE_DEFAULT: Self = Self(MAP_HUGE_SHIFT);
    /// Predefined `MAP_HUGE_2MB` mask,
    pub const HUGE_2MB: Self = Self(libc::MAP_HUGE_2MB);
    /// Predefined `MAP_HUGE_1GB` mask,
    pub const HUGE_1GB: Self = Self(libc::MAP_HUGE_1GB);
    
    /// Calculate a `MAP_HUGE_*` flag from a size (in kB).
    #[inline(always)] 
    pub const fn calculate(kilobytes: NonZeroUsize) -> Self
    {
	Self((log2(kilobytes.get()) << (MAP_HUGE_SHIFT as usize)) as c_int)
    }

    /// Get the `MAP_HUGE_*` mask.
    #[inline(always)] 
    pub const fn get_mask(self) -> c_int
    {
	self.0
    }
}

impl From<MapHugeFlag> for c_int
{
    fn from(from: MapHugeFlag) -> Self
    {
	from.0
    }
}


#[derive(Default, Clone, Copy)]
pub enum HugePage {
    /// A staticly presented `MAP_HUGE_*` flag. See `MapHugeFlag` for details.
    Static(MapHugeFlag),
    /// A dynamically calculated `MAP_HUGE_*` flag from an arbitrary size *in kB*.
    ///
    /// # Safety
    /// The kernel must actually support huge-pages of this size.
    Dynamic{ kilobytes: usize },
    /// The smallest huge-page size on the system
    #[default]
    Smallest,
    /// The largest huge-page size on the system 
    Largest,
    /// Use a callback function to select the huge-page size (*in kB*) from an *ordered* (lowest to highest) enumeration of all available on the system.
    //TODO: Remember to order the HUGEPAGE_LOCATION parsing results before passing them to this!
    Selected(for<'r> fn (&'r [usize]) -> &'r usize),
}

impl hash::Hash for HugePage {
    #[inline] 
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
	mem::discriminant(self).hash(state);
	match self {
	    Self::Static(hpf) => hpf.hash(state),
	    Self::Dynamic { kilobytes } => kilobytes.hash(state),
	    Self::Selected(func) => ptr::hash(func as *const _, state),
	    _ => (),
	};
    }
}


impl fmt::Debug for HugePage
{
    #[inline] 
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
	f.debug_tuple("HugePage")
	    .field({
		let v: &dyn fmt::Debug = match &self {
		    Self::Static(ref huge) => huge,
		    Self::Dynamic { ref kilobytes } => kilobytes,
		    Self::Smallest => &"<smallest>",
		    Self::Largest => &"<largest>",
		    Self::Selected(_) => &"<selector>",
		};
		v
	    })
	    .finish()
    }
}


impl Eq for HugePage {}
impl PartialEq for HugePage
{
    #[inline] 
    fn eq(&self, other: &Self) -> bool
    {
	 match (self, other) {
	    (Self::Static(hpf), Self::Static(hpf2)) => hpf == hpf2,
	    (Self::Dynamic { kilobytes }, Self::Dynamic { kilobytes: kilobytes2 }) => kilobytes == kilobytes2,
	    (Self::Selected(func), Self::Selected(func2)) => ptr::eq(func, func2),
	    _ => mem::discriminant(self) == mem::discriminant(other),
	}
    }
}

impl HugePage
{
    pub fn compute_huge(&self) -> Option<MapHugeFlag>
    {
	todo!("TODO: copy `utf8encode`'s `compute_huge_flag()` -> pub fn compute_flag(&self) -> Option<MapHugeFlag>;")
    }
    //TODO: ^
}

//TODO: implement `memfd`'s hugetlb interface from `utf8encode` here.
