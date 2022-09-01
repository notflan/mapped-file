//! All flags for controlling a `MappedFile<T>`.
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
    const fn get_ms(self) -> c_int
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
    const fn get_madv(self) -> c_int
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
