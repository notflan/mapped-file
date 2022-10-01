# `MemoryFile<T>`: Map over any file object

A safe and ergonomic `mmap()` wrapper for arbitrary file-descriptor handles.

__NOTE__: Working release, but still in development.

## Usage

`MemoryFile<T>` can be used to consume any type `T` that implements `AsRawFd`, form a mapping over that file-descriptor, and then unmap the memory before the `T` itself is dropped (which can be a reference or value.)
The `MemoryFile<T>` can also be consumed back into the `T`, unmapping (and optionally syncing) the memory in the process.

### Examples

A function mapping file memory working on arbitrary file-descriptor holding objects.

```rust
pub fn files_equal<T: ?Sized, U: ?Sized>(file1: &T, file2: &U, size: usize) -> io::Result<bool>
	where T: AsRawFd,
		  U: AsRawFd
{
	let file1 = MappedFile::try_new(file1, size, Perm::Readonly, Flags::Private)?.with_advice(Advice::Sequential)?;
	let file2 = MappedFile::try_new(file2, size, Perm::Readonly, Flags::Private)?.with_advice(Advice::Sequential)?;
	Ok(&file1[..] == &file2[..])
}
```

Although, it is probably a better pattern to allow the caller to handle the mapping, and the callee to take any kind of mapping like so:

``` rust
pub fn files_equal<T: ?Sized, U: ?Sized>(file1: &MappedFile<T>, file2: &MappedFile<U>) -> bool
	where T: AsRawFd,
		  U: AsRawFd
{
	&file1[..] == &file2[..]
}

```

However, `MappedFile<T>` also implements `Borrow<[u8]>`, so any `&MappedFile<T>` can be passed to any function as `AsRef<[u8]>` too.

# License
MIT
