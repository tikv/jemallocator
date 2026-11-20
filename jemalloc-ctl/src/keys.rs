//! Typed keys in the jemalloc control namespace.

use crate::error::Result;
use crate::{fmt, ops, raw};
use core::{marker::PhantomData, str};

/// The name of a jemalloc control key with value type `T`.
///
/// Built-in keys such as [`crate::epoch`] provide a name through their
/// `name()` method. Use [`Name::new`] to construct a name for a custom key.
///
/// ```
/// # #[global_allocator]
/// # static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
/// use tikv_jemalloc_ctl::{Access, Name};
///
/// // SAFETY: arenas.nbins returns a c_uint and rejects writes.
/// let nbins = unsafe { Name::<libc::c_uint>::new(b"arenas.nbins\0") };
/// let mib = nbins.mib::<2>().unwrap();
/// assert_eq!(nbins.read().unwrap(), mib.read().unwrap());
/// ```
///
/// ```compile_fail
/// use tikv_jemalloc_ctl::{epoch, Access};
///
/// let _: bool = epoch::name().read().unwrap();
/// ```
#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Name<'a, T> {
    bytes: &'a [u8],
    value: PhantomData<fn(T) -> T>,
}

impl<'a, T> Name<'a, T> {
    /// Creates a typed name from a NUL-terminated byte string.
    ///
    /// # Safety
    ///
    /// If this function returns, the name must identify a complete jemalloc
    /// key of value type `T`, even if the running build lacks that key.
    /// Access through the name or its MIB must be safe from any thread for
    /// all inputs accepted by [`Access<T>`]. Reads and updates require a
    /// pure output buffer as described in [`raw::read()`]. Any returned
    /// string pointer must refer to an immutable NUL-terminated string
    /// valid for the entire program.
    ///
    /// # Panics
    ///
    /// Panics unless `bytes` ends in a single NUL and has no interior NULs.
    ///
    /// ```compile_fail
    /// use tikv_jemalloc_ctl::Name;
    ///
    /// let _ = Name::<u64>::new(b"epoch\0");
    /// ```
    pub const unsafe fn new(bytes: &'a [u8]) -> Self {
        assert!(
            !bytes.is_empty(),
            "cannot create Name from empty byte-string"
        );
        let end = bytes.len() - 1;
        assert!(bytes[end] == 0, "jemalloc key is not NUL-terminated");
        let mut i = 0;
        while i < end {
            assert!(bytes[i] != 0, "jemalloc key contains an interior NUL");
            i += 1;
        }
        Self {
            bytes,
            value: PhantomData,
        }
    }

    pub(crate) const fn component_count(&self) -> usize {
        let mut count = 1;
        let mut i = 0;
        while i < self.bytes.len() {
            if self.bytes[i] == b'.' {
                count += 1;
            }
            i += 1;
        }
        count
    }

    /// Returns the complete MIB for this key.
    ///
    /// # Panics
    ///
    /// Panics if `N` differs from the number of period-separated components
    /// in the name.
    pub fn mib<const N: usize>(&self) -> Result<Mib<T, N>> {
        assert_eq!(
            N,
            self.component_count(),
            "MIB length must match the number of key components"
        );
        let mut indices = [0; N];
        raw::name_to_mib(self.bytes, &mut indices)?;
        // SAFETY: The lookup returned the complete MIB for this typed key.
        Ok(unsafe { Mib::from_raw(indices) })
    }

    /// Returns the NUL-terminated bytes of the name.
    pub const fn as_bytes(&self) -> &'a [u8] {
        self.bytes
    }
}

impl<T> fmt::Debug for Name<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl<T> fmt::Display for Name<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(str::from_utf8(self.bytes).unwrap())
    }
}

/// The Management Information Base of a jemalloc key with value type `T`.
///
/// A MIB obtained from [`Name::mib`] can be reused without looking up the
/// name again. Use [`Mib::from_raw`] to use indices from an external source.
///
/// ```compile_fail
/// use tikv_jemalloc_ctl::epoch;
///
/// let mut mib = epoch::name().mib::<1>().unwrap();
/// mib[0] = 0;
/// ```
#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Mib<T, const N: usize> {
    indices: [usize; N],
    value: PhantomData<fn(T) -> T>,
}

impl<T, const N: usize> Mib<T, N> {
    /// Creates a typed MIB from its raw indices.
    ///
    /// # Safety
    ///
    /// The indices must form a complete, valid MIB for a key in the running
    /// jemalloc instance. The key must satisfy the type and access
    /// requirements of [`Name::new`] for `T`.
    pub const unsafe fn from_raw(indices: [usize; N]) -> Self {
        Self {
            indices,
            value: PhantomData,
        }
    }
}

impl<T, const N: usize> AsRef<[usize]> for Mib<T, N> {
    fn as_ref(&self) -> &[usize] {
        &self.indices
    }
}

impl<T, const N: usize> ops::Index<usize> for Mib<T, N> {
    type Output = usize;

    fn index(&self, index: usize) -> &usize {
        &self.indices[index]
    }
}

/// Typed access to a jemalloc key. The key determines which operations it
/// supports and returns an error for unsupported operations.
pub trait Access<T> {
    /// Reads the current value.
    fn read(&self) -> Result<T>;
    /// Writes a new value.
    ///
    /// For string keys, returns `EINVAL` if `value` lacks a trailing NUL.
    fn write(&self, value: T) -> Result<()>;
    /// Updates the key and returns the value defined by that key.
    ///
    /// For example, [`crate::epoch`] refreshes the allocator statistics and
    /// returns the refreshed epoch. For string keys, returns `EINVAL` if
    /// `value` lacks a trailing NUL.
    fn update(&self, value: T) -> Result<T>;
}

macro_rules! impl_access {
    ($ty:ty, $read:ident, $write:ident, $update:ident, $read_mib:ident, $write_mib:ident, $update_mib:ident) => {
        impl Access<$ty> for Name<'_, $ty> {
            fn read(&self) -> Result<$ty> {
                unsafe { raw::$read(self.bytes) }
            }

            fn write(&self, value: $ty) -> Result<()> {
                unsafe { raw::$write(self.bytes, value) }
            }

            fn update(&self, value: $ty) -> Result<$ty> {
                unsafe { raw::$update(self.bytes, value) }
            }
        }

        impl<const N: usize> Access<$ty> for Mib<$ty, N> {
            fn read(&self) -> Result<$ty> {
                unsafe { raw::$read_mib(&self.indices) }
            }

            fn write(&self, value: $ty) -> Result<()> {
                unsafe { raw::$write_mib(&self.indices, value) }
            }

            fn update(&self, value: $ty) -> Result<$ty> {
                unsafe { raw::$update_mib(&self.indices, value) }
            }
        }
    };
}

impl_access!(u32, read, write, update, read_mib, write_mib, update_mib);
impl_access!(u64, read, write, update, read_mib, write_mib, update_mib);
impl_access!(isize, read, write, update, read_mib, write_mib, update_mib);
impl_access!(usize, read, write, update, read_mib, write_mib, update_mib);
impl_access!(bool, read, write, update, read_mib, write_mib, update_mib);
impl_access!(
    &'static [u8],
    read_str,
    write_str,
    update_str,
    read_str_mib,
    write_str_mib,
    update_str_mib
);

impl Access<&'static str> for Name<'_, &'static str> {
    fn read(&self) -> Result<&'static str> {
        let value = unsafe { raw::read_str(self.bytes)? };
        Ok(str::from_utf8(value).unwrap())
    }

    fn write(&self, value: &'static str) -> Result<()> {
        unsafe { raw::write_str(self.bytes, value.as_bytes()) }
    }

    fn update(&self, value: &'static str) -> Result<&'static str> {
        let value = unsafe { raw::update_str(self.bytes, value.as_bytes())? };
        Ok(str::from_utf8(value).unwrap())
    }
}

impl<const N: usize> Access<&'static str> for Mib<&'static str, N> {
    fn read(&self) -> Result<&'static str> {
        let value = unsafe { raw::read_str_mib(&self.indices)? };
        Ok(str::from_utf8(value).unwrap())
    }

    fn write(&self, value: &'static str) -> Result<()> {
        unsafe { raw::write_str_mib(&self.indices, value.as_bytes()) }
    }

    fn update(&self, value: &'static str) -> Result<&'static str> {
        let value =
            unsafe { raw::update_str_mib(&self.indices, value.as_bytes())? };
        Ok(str::from_utf8(value).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::{Access, Name};
    use crate::error::cvt;

    #[test]
    fn bool_rw() {
        let name = unsafe { Name::<bool>::new(b"thread.tcache.enabled\0") };
        let tcache = name.read().unwrap();
        let new_tcache = !tcache;

        name.write(new_tcache).unwrap();
        let mib = name.mib::<3>().unwrap();
        assert_eq!(mib.read().unwrap(), new_tcache);
    }

    #[test]
    fn u32_r() {
        let name = unsafe { Name::<u32>::new(b"arenas.bin.0.nregs\0") };
        let mib = name.mib::<4>().unwrap();
        assert_eq!(mib.read().unwrap(), name.read().unwrap());
    }

    #[test]
    fn size_t_r() {
        let name =
            unsafe { Name::<libc::size_t>::new(b"arenas.lextent.0.size\0") };
        let mib = name.mib::<4>().unwrap();
        assert_eq!(mib.read().unwrap(), name.read().unwrap());
    }

    #[test]
    fn ssize_t_rw() {
        let name =
            unsafe { Name::<libc::ssize_t>::new(b"arenas.dirty_decay_ms\0") };
        let value = name.read().unwrap();
        name.write(value).unwrap();
        let mib = name.mib::<2>().unwrap();
        assert_eq!(mib.read().unwrap(), value);
    }

    #[test]
    fn str_rw() {
        let name = unsafe { Name::<&'static [u8]>::new(b"arena.0.dss\0") };
        let dss = name.read().unwrap();
        name.write(dss).unwrap();
        let mib = name.mib::<3>().unwrap();
        let mib_dss = mib.read().unwrap();
        mib.write(mib_dss).unwrap();
        assert_eq!(dss, mib_dss);
    }

    #[test]
    fn string_write_and_update_errors() {
        let name = crate::version::name();
        let mib = name.mib::<1>().unwrap();
        for (value, errno) in [
            ("", libc::EINVAL),
            ("unterminated", libc::EINVAL),
            ("terminated\0", libc::EPERM),
        ] {
            assert_eq!(name.write(value), cvt(errno));
            assert_eq!(mib.write(value), cvt(errno));
            assert_eq!(name.update(value).map(|_| ()), cvt(errno));
            assert_eq!(mib.update(value).map(|_| ()), cvt(errno));
        }
    }

    #[test]
    #[should_panic(expected = "MIB length must match")]
    fn rejects_partial_mib() {
        let _ = crate::arenas::narenas::name().mib::<1>();
    }
}
