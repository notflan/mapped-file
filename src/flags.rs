//! All flags for controlling a `MappedFile<T>`.
use super::*;
use libc::c_int;

/// Permissions for the mapped pages.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Default)]
pub enum Perm
{
#[default]
    ReadWrite,
    Readonly,
    Writeonly,
    RX,
    WRX,
}

/// Flags for mapping a file descriptor.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Default)]
pub enum Flags
{
#[default]
    Shared,
    Private,
}

impl Flags
{
    /// Add these flags to another `MapFlags` provider's mask.
    ///
    /// # Safety
    /// The caller *should* ensure there are no conflicting flags present in the bitwise OR of `self` and `flags`'s respective masks; if there are, then `mmap()` may fail. This will not result in unexpected mapping behaviour, but will cause an error.
    ///
    /// However, the caller **must** ensure there are no *overlapping* bits in the resulting mask, as that may produce a valid but unexpected combined mask.
    ///
    /// # `hugetlb` support
    /// For adding huge-page mapping flags to these, use `with_hugetlb()` instead.
    #[inline] 
    pub unsafe fn chain_with(self, flags: impl MapFlags) -> impl MapFlags
    {
	struct Chained<T: ?Sized>(Flags, T);

	unsafe impl<T: ?Sized> MapFlags for Chained<T>
	where T: MapFlags
	{
	    #[inline(always)] 
	    fn get_mmap_flags(&self) -> c_int {
		self.0.get_flags() | self.1.get_mmap_flags()
	    }
	}

	Chained(self, flags)
    }
    /// Add huge-page info to the mapping flags for this `MappedFile<T>` instance.
    ///
    /// # Returns
    /// An opaque type that combines the flags of `self` with those computed by `hugetlb`.
    #[inline] 
    pub const fn with_hugetlb(self, hugetlb: HugePage) -> impl MapFlags + Send + Sync + 'static
    {
	#[derive(Debug)]
	struct HugeTLBFlags(Flags, HugePage);
	unsafe impl MapFlags for HugeTLBFlags
	{
	    #[inline(always)]
	    fn get_mmap_flags(&self) -> c_int {
		self.0.get_flags() | self.1.compute_huge().map(MapHugeFlag::get_mask).unwrap_or(0)
	    }
	}

	HugeTLBFlags(self, hugetlb)
    }
}

/// Any type implementing this trait can be passed to `MappedFile<T>`'s `try_/new()` method to provide flags directly for `mmap()`.
/// Usually, the enum `Flags` should be used for this, but for HUGETLB configurations, or used-defined `MAP_FIXED` usages, it can be used on other types.
///
/// This trait is also implemented on `()`, which will just return `Flags::default()`'s implementation of it.
///
/// # Safety
/// This trait is marked `unsafe` as invalid memory mapping configurations can cause invalid or undefined behaviour that is unknown to `MappedFile<T>`.
pub unsafe trait MapFlags
{
    fn get_mmap_flags(&self) -> c_int;
}

unsafe impl MapFlags for ()
{
    #[inline]
    fn get_mmap_flags(&self) -> c_int {
	Flags::default().get_flags()
    }
}

unsafe impl MapFlags for Flags
{
    #[inline(always)]
    fn get_mmap_flags(&self) -> c_int {
	self.get_flags()
    }
}

impl Flags
{
#[inline(always)]
    pub(super) const fn get_flags(self) -> c_int
    {
        use libc::{
            MAP_SHARED,
            MAP_PRIVATE,
        };
        match self {
            Self::Shared => MAP_SHARED,
            Self::Private => MAP_PRIVATE,
        }
    }
#[inline(always)]
    pub(super) const fn requires_write_access(&self) -> bool
    {
        match self {
            Self::Shared => true,
            _ => false
        }
    }
}

impl Perm
{
#[inline(always)]
    pub(super) const fn get_prot(self) -> c_int
    {
        use libc::{
            PROT_READ, PROT_WRITE, PROT_EXEC,
        };
        match self {
            Self::ReadWrite => PROT_READ | PROT_WRITE,
            Self::Readonly => PROT_READ,
            Self::Writeonly => PROT_WRITE,
            Self::RX => PROT_READ | PROT_EXEC,
            Self::WRX => PROT_READ | PROT_WRITE | PROT_EXEC,
        }
    }
#[inline(always)]
    pub(super) const fn open_rw(&self, flags: Flags) -> (bool, bool)
    {
        let wr = flags.requires_write_access();
        match self {
            Self::ReadWrite | Self::WRX => (true, wr),
            Self::Readonly | Self::RX => (true, false),
            Self::Writeonly => (false, wr),
        }
    }
}

/// Options for flushing a mapping. These will control how the `msync()` is called.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Default)]
pub enum Flush
{
#[default]
    Wait,
    Async,
    Invalidate,
    InvalidateAsync,
}

impl Flush
{
#[inline(always)]
    pub(super) const fn get_ms(self) -> c_int
    {
        use libc::{
            MS_SYNC, MS_ASYNC,
            MS_INVALIDATE,
        };
        match self {
            Self::Wait => MS_SYNC,
            Self::Async => MS_ASYNC,
            Self::Invalidate => MS_SYNC | MS_INVALIDATE,
            Self::InvalidateAsync => MS_ASYNC | MS_INVALIDATE,
        }
    }
}

/// Advice to the kernel about how to load the mapped pages. These will control `madvise()`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Default)]
pub enum Advice {
#[default]
    Normal,
    Sequential,
    RandomAccess,
}

impl Advice
{
#[inline(always)]
    pub(crate) const fn get_madv(self) -> c_int
    {
        use libc::{
            MADV_NORMAL,
            MADV_SEQUENTIAL,
            MADV_RANDOM,
        };
        match self {
            Self::Normal => MADV_NORMAL,
            Self::Sequential => MADV_SEQUENTIAL,
            Self::RandomAccess => MADV_RANDOM,
        }
    }
}
