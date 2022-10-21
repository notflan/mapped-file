//! Huge-page interface for `MappedFile<T>` and `MemoryFile`.
use super::*;
use std::{
    mem,
    hash,
    num::NonZeroUsize,
    fs,
    path::{Path, PathBuf},
    fmt, error,
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

/// Error for when `HugePage::compute_huge()` fails.
#[derive(Debug)]
pub struct HugePageCalcErr(());

impl TryFrom<HugePage> for MapHugeFlag
{
    type Error = HugePageCalcErr;

    #[inline] 
    fn try_from(from: HugePage) -> Result<Self, Self::Error>
    {
	from.compute_huge().ok_or(HugePageCalcErr(()))
    }
}


impl error::Error for HugePageCalcErr{}
impl fmt::Display for HugePageCalcErr
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
	f.write_str("Invalid huge-page specification")
    }
}


impl Default for MapHugeFlag
{
    #[inline] 
    fn default() -> Self {
	Self(MAP_HUGE_SHIFT)
    }
}
#[inline(always)]
const fn log2(n: usize) -> usize
{
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

    /// Attempt to calculate `MAP_HUGE_*` flag from a size (in kB).
    #[inline]
    pub const fn try_calculate(kilobytes: usize) -> Option<Self>
    {
	match kilobytes {
	    0 => None,
	    kilobytes => {
		if let Some(shift) = log2(kilobytes).checked_shl(MAP_HUGE_SHIFT as u32) {
		    if shift <= c_int::MAX as usize {
			return Some(Self(shift as c_int));
		    }
		}
		None
	    }
	}
    }

    /// Attempt to calculate `MAP_HUGE_*`, or use `HUGE_DEFAULT` on failure.
    ///
    /// # Note
    /// If `kilobytes` is `0`, or there is a calculation overflow, then `HUGE_DEFAULT` is returned.
    #[inline] 
    pub const fn calculate_or_default(kilobytes: usize) -> Self
    {
	match Self::try_calculate(kilobytes) {
	    None => Self::HUGE_DEFAULT,
	    Some(x) => x,
	}
    }

    /// Check if this is the smallest huge-page size the kernel supports.
    #[inline] 
    pub const fn is_default(&self) -> bool
    {
	self.0 == Self::HUGE_DEFAULT.0
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
    #[inline] 
    fn from(from: MapHugeFlag) -> Self
    {
	from.0
    }
}

/// Provides an arbitrary huge-page size and mapping flag for that size.
///
/// Can store or create a `MAP_HUGE_*` flag for use with `mmap()`, (`MappedFile`) or `memfd_create()` (`file::MemoryFile::with_hugetlb()`)
///
/// # Usage
/// Main usage is for generating a `MapHugeFlag` via `compute_huge()`. This function may fail (rarely), so a `TryInto` impl exists for `MapHugeFlag` as well.
#[derive(Default, Clone, Copy)]
pub enum HugePage {
    /// A staticly presented `MAP_HUGE_*` flag. See `MapHugeFlag` for details.
    Static(MapHugeFlag),
    /// A dynamically calculated `MAP_HUGE_*` flag from an arbitrary size *in kB*.
    ///
    /// # Safety
    /// The kernel must actually support huge-pages of this size.
    ///
    /// If `kilobytes` is 0, or an overflow in calculation happens, then this is identical to `Smallest`.
    Dynamic{ kilobytes: usize },
    /// The smallest huge-page size on the system
    #[default]
    Smallest,
    /// The largest huge-page size on the system 
    Largest,
    /// Use a callback function to select the huge-page size (*in kB*) from an *ordered* (lowest to highest) enumeration of all available on the system.
    Selected(for<'r> fn (&'r [usize]) -> Option<&'r usize>),
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
    /// Compute the `MapHugeFlag` from this huge-page specification.
    ///
    /// # Returns
    /// * `None` - If there was an error in computing the correct flag.
    /// * `Some` - If the computation was successful.
    /// 
    /// # Panics
    /// In debug builds, if scanning the system for huge-pages fails after `SYSTEM_HUGEPAGES` has already failed.
    #[inline]  // This call is recursive, but also can be large for variant `Selected`, which we have factored out into a non-inline local function. All other variants are small enough for this to be okay.
    pub fn compute_huge(self) -> Option<MapHugeFlag>
    {
	use HugePage::*;
	match self {
	    Dynamic { kilobytes: 0 } |
	    Smallest |
	    Static(MapHugeFlag::HUGE_DEFAULT) => Some(MapHugeFlag::HUGE_DEFAULT),
	    Static(mask) => Some(mask),
	    Dynamic { kilobytes } => {
		MapHugeFlag::try_calculate(kilobytes) //XXX: Should we use `calculate_or_default()` here?
	    },
	    Largest => Self::Selected(|sizes| sizes.iter().max()).compute_huge(),
	    Selected(func) => {
		// Factored out into a non-`inline` function since it's the only one doing actual work, and allows the parent function to be `inline` without bloating to much
		fn compute_selected(func: for<'r> fn (&'r [usize]) -> Option<&'r usize>) -> Option<MapHugeFlag>
		{
		    use std::borrow::Cow;
		    let mask = match SYSTEM_HUGEPAGE_SIZES.as_ref() {
			Ok(avail) => Cow::Borrowed(&avail[..]),
			Err(_) => {
			    // Attempt to re-scan the system. Fail if scan fails.
			    #[cold]
			    fn rescan() -> io::Result<Vec<usize>>
			    {
				scan_hugepages().and_then(|x| x.into_iter().collect())
			    }
			    let v = rescan();
			    let mut v = if cfg!(debug_assertions) {
				v.expect("Failed to compute available hugetlb sizes")
			    } else {
				v.ok()?
			    };
			    v.sort_unstable();
			    Cow::Owned(v)
			},
		    };

		    match func(mask.as_ref()) {
			Some(mask) => Dynamic { kilobytes: *mask }.compute_huge(),
			None => Some(MapHugeFlag::HUGE_DEFAULT),
		    }
		}
		compute_selected(func)
	    },
	}
    }
}

lazy_static! {
    /// A persistent invocation of `scan_hugepages()`.
    pub(crate) static ref SYSTEM_HUGEPAGE_SIZES: io::Result<Vec<usize>> = {
	let mut val: io::Result<Vec<usize>> = scan_hugepages().and_then(|x| x.into_iter().collect());
	if let Ok(ref mut arr) = val.as_mut() {
	    arr.sort_unstable();
	};
	val
    };

    /// A list of all availble huge-page flags if enumeration of them is possible.
    ///
    /// This is created from a persistent invocation of `scan_hugepages()`.
    pub static ref SYSTEM_HUGEPAGES: io::Result<Vec<MapHugeFlag>> =
	SYSTEM_HUGEPAGE_SIZES.as_ref()
	.map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, format!("SYSTEM_HUGEPAGES failed with error {err}")))
	.map(|vec| vec.iter().map(|&size| MapHugeFlag::calculate_or_default(size)).collect());
}

/// Scan the system for available huge-page sizes (in kB).
///
/// # Returns
/// If reading the directory `HUGEPAGE_LOCATION` fails, then the error is returned.
/// Otherwise, an iterator over each item in this location, parsed for its size, is returned.
/// If reading an entry fails, an error is returned.
///
/// If an entry is not parsed correctly, then it is skipped.
pub fn scan_hugepages() -> io::Result<impl IntoIterator<Item=io::Result<usize>> + Send + Sync + 'static>
{
    let path = Path::new(HUGEPAGE_LOCATION);
    let dir = fs::read_dir(path)?;

    #[derive(Debug)]
    struct FilteredIterator(fs::ReadDir);
    
    impl Iterator for FilteredIterator
    {
	type Item = io::Result<usize>;
	fn next(&mut self) -> Option<Self::Item> {
	    loop {
		break if let Some(next) = self.0.next() {
		    let path = match next {
			Ok(next) => next.file_name(),
			Err(err) => return Some(Err(err)),
		    };
		    let kbs = if let Some(dash) = memchr::memchr(b'-', path.as_bytes()) {
			let name = &path.as_bytes()[(dash+1)..];
			if let Some(k_loc) = memchr::memrchr(b'k', &name) {
			    &name[..k_loc]
			} else {
			    continue
			}
		    } else {
			continue
		    };
		    let kb = if let Ok(kbs) = std::str::from_utf8(kbs) {
			kbs.parse::<usize>().ok()
		    } else {
			continue
		    };
		    match kb {
			None => continue,
			valid => valid.map(Ok)
		    }
		} else {
		    None
		}
	    }
	}
    }
    
    Ok(FilteredIterator(dir))
}
