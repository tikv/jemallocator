//! Typed keys in the jemalloc control namespace.

use crate::error::{Error, Result};
use crate::{fmt, ops, raw};
use core::{ffi::CStr, marker::PhantomData, str};

// Matches CTL_MAX_DEPTH in the bundled jemalloc. The extra slot lets us
// distinguish a complete lookup from a truncated MIB.
const MAX_MIB_DEPTH: usize = 7;

/// The name of a jemalloc control key with value type `T`.
///
/// Built-in keys such as [`crate::epoch`] provide a name through their
/// `name()` method. For a custom key, use [`Name::new`] with bytes or
/// [`Name::from_c_str`] with an existing C string.
///
/// # Example
///
/// ```
/// # #[global_allocator]
/// # static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
/// # fn main() -> tikv_jemalloc_ctl::Result<()> {
/// use tikv_jemalloc_ctl::{Access, Name};
///
/// // SAFETY: arenas.nbins returns a c_uint and rejects writes.
/// let nbins = unsafe { Name::<libc::c_uint>::new(b"arenas.nbins\0") }?;
/// let mib = nbins.mib()?;
/// assert_eq!(nbins.read()?, mib.read()?);
/// # Ok(())
/// # }
/// ```
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Name<'a, T> {
    name: &'a CStr,
    value: PhantomData<fn(T) -> T>,
}

impl<'a, T> Name<'a, T> {
    /// Creates a typed name from a NUL-terminated byte string.
    ///
    /// # Errors
    ///
    /// Returns `EINVAL` if the name is empty or `bytes` does not end in
    /// a single NUL with no interior NULs.
    ///
    /// # Safety
    ///
    /// On success, the name must identify a complete jemalloc
    /// key of value type `T`, even if the running build lacks that key.
    /// Access through the name or its MIB must be safe from any thread for
    /// all inputs accepted by [`Access<T>`]. Reads and updates require a
    /// pure output buffer as described in [`raw::read()`]. Any non-null
    /// string pointer returned by the key must refer to an immutable
    /// NUL-terminated string valid for the entire program.
    pub const unsafe fn new(bytes: &'a [u8]) -> Result<Self> {
        match CStr::from_bytes_with_nul(bytes) {
            Ok(name) => Self::from_c_str(name),
            Err(_) => Err(Error::INVALID_ARGUMENT),
        }
    }

    /// Creates a typed name from an existing C string.
    ///
    /// # Errors
    /// Returns `EINVAL` if `name` is empty.
    ///
    /// # Safety
    /// The key must satisfy the type and access requirements of [`Self::new`].
    pub const unsafe fn from_c_str(name: &'a CStr) -> Result<Self> {
        if name.is_empty() {
            return Err(Error::INVALID_ARGUMENT);
        }
        Ok(Self {
            name,
            value: PhantomData,
        })
    }

    // Computes the array length in the built-in MIB declarations.
    pub(crate) const fn component_count(&self) -> usize {
        let mut bytes = self.name.to_bytes();
        let mut count = 1;
        while let Some((&byte, remaining)) = bytes.split_first() {
            if byte == b'.' {
                count += 1;
            }
            bytes = remaining;
        }
        count
    }

    /// Returns the complete MIB for this key.
    ///
    /// # Errors
    ///
    /// Returns `EINVAL` if the lookup yields no components or if the key
    /// exceeds the maximum supported MIB depth of seven components.
    pub fn mib(&self) -> Result<Mib<T>> {
        let mut mib = Mib {
            indices: [0; MAX_MIB_DEPTH + 1],
            len: 0,
            value: PhantomData,
        };
        let len = raw::name_to_mib_cstr(self.name, &mut mib.indices)?;
        if len == 0 || len > MAX_MIB_DEPTH {
            return Err(Error::INVALID_ARGUMENT);
        }
        mib.len = len;
        Ok(mib)
    }

    /// Returns the NUL-terminated bytes of the name.
    pub const fn as_bytes(&self) -> &'a [u8] {
        self.name.to_bytes_with_nul()
    }

    pub(crate) const fn as_c_str(&self) -> &'a CStr {
        self.name
    }
}

impl<T> fmt::Debug for Name<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl<T> fmt::Display for Name<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.name.to_str() {
            Ok(name) => f.write_str(name),
            Err(_) => {
                fmt::Display::fmt(&self.name.to_bytes().escape_ascii(), f)
            }
        }
    }
}

/// The Management Information Base of a jemalloc key with value type `T`.
///
/// A MIB obtained from [`Name::mib`] can be reused without looking up the
/// name again. Its indices are read-only. Use [`Mib::from_raw`] to construct
/// a MIB from indices obtained elsewhere.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Mib<T> {
    indices: [usize; MAX_MIB_DEPTH + 1],
    len: usize,
    value: PhantomData<fn(T) -> T>,
}

impl<T> Mib<T> {
    /// Creates a typed MIB from its raw indices.
    ///
    /// # Errors
    ///
    /// Returns `EINVAL` if the slice is empty or contains more than seven
    /// components.
    ///
    /// # Safety
    ///
    /// If accepted, the indices must form a complete, valid MIB for a key in
    /// the running jemalloc instance. The key must satisfy the type and access
    /// requirements of [`Name::new`] for `T`.
    pub const unsafe fn from_raw(indices: &[usize]) -> Result<Self> {
        let len = indices.len();
        if len == 0 || len > MAX_MIB_DEPTH {
            return Err(Error::INVALID_ARGUMENT);
        }
        let mut mib = Self {
            indices: [0; MAX_MIB_DEPTH + 1],
            len,
            value: PhantomData,
        };
        let mut i = 0;
        while i < len {
            mib.indices[i] = indices[i];
            i += 1;
        }
        Ok(mib)
    }

    fn as_mib_ref(&self) -> MibRef<'_, T> {
        // SAFETY: The owned handle holds a complete MIB for this value type.
        unsafe { MibRef::from_raw(self.as_ref()) }
    }
}

impl<T> fmt::Debug for Mib<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Mib")
            .field("indices", &self.as_ref())
            .finish()
    }
}

impl<T> AsRef<[usize]> for Mib<T> {
    fn as_ref(&self) -> &[usize] {
        &self.indices[..self.len]
    }
}

impl<T> ops::Index<usize> for Mib<T> {
    type Output = usize;

    fn index(&self, index: usize) -> &usize {
        &self.as_ref()[index]
    }
}

/// Typed access to a jemalloc key. The key determines which operations it
/// supports and returns an error for unsupported operations.
///
/// Boolean access uses jemalloc's C boolean type. On MSVC targets this is
/// a C `int`, with nonzero values mapped to `true` and writes using `0` or `1`.
/// For string values, `&'static CStr` can be used directly. String reads
/// borrow the value and writes pass its pointer to jemalloc.
pub trait Access<T> {
    /// Reads the current value.
    ///
    /// # Errors
    ///
    /// Reading an `&str` returns `EILSEQ` if the result is not UTF-8.
    fn read(&self) -> Result<T>;
    /// Writes a new value.
    ///
    /// For `&[u8]` and `&str` values, returns `EINVAL` if `value` lacks a trailing NUL.
    fn write(&self, value: T) -> Result<()>;
    /// Updates the key and returns the value defined by that key.
    ///
    /// For example, [`crate::epoch`] refreshes the allocator statistics and
    /// returns the refreshed epoch.
    ///
    /// # Errors
    ///
    /// For `&[u8]` and `&str` values, returns `EINVAL` if `value` lacks a
    /// trailing NUL. For `&str`, returns `EILSEQ` if the result is not UTF-8.
    /// The write may already have taken effect when `EILSEQ` is returned.
    fn update(&self, value: T) -> Result<T>;
}

pub(crate) struct MibRef<'a, T> {
    indices: &'a [usize],
    value: PhantomData<fn(T) -> T>,
}

impl<'a, T> MibRef<'a, T> {
    /// # Safety
    /// The indices must satisfy the type and access requirements of
    /// [`Mib::from_raw`].
    pub(crate) unsafe fn from_raw(indices: &'a [usize]) -> Self {
        Self {
            indices,
            value: PhantomData,
        }
    }
}

impl<T> Access<T> for Mib<T>
where
    for<'a> MibRef<'a, T>: Access<T>,
{
    fn read(&self) -> Result<T> {
        self.as_mib_ref().read()
    }

    fn write(&self, value: T) -> Result<()> {
        self.as_mib_ref().write(value)
    }

    fn update(&self, value: T) -> Result<T> {
        self.as_mib_ref().update(value)
    }
}

macro_rules! impl_access {
    ($ty:ty) => {
        impl_access!(
            $ty,
            read_cstr,
            write_cstr,
            update_cstr,
            read_mib,
            write_mib,
            update_mib
        );
    };
    ($ty:ty, $read:ident, $write:ident, $update:ident, $read_mib:ident, $write_mib:ident, $update_mib:ident $(; $decode:path)?) => {
        impl Access<$ty> for Name<'_, $ty> {
            fn read(&self) -> Result<$ty> {
                unsafe { raw::$read(self.name) }$(.map($decode))?
            }

            fn write(&self, value: $ty) -> Result<()> {
                unsafe { raw::$write(self.name, value) }
            }

            fn update(&self, value: $ty) -> Result<$ty> {
                unsafe { raw::$update(self.name, value) }$(.map($decode))?
            }
        }

        impl Access<$ty> for MibRef<'_, $ty> {
            fn read(&self) -> Result<$ty> {
                unsafe { raw::$read_mib(self.indices) }$(.map($decode))?
            }

            fn write(&self, value: $ty) -> Result<()> {
                unsafe { raw::$write_mib(self.indices, value) }
            }

            fn update(&self, value: $ty) -> Result<$ty> {
                unsafe { raw::$update_mib(self.indices, value) }$(.map($decode))?
            }
        }
    };
}

impl_access!(u32);
impl_access!(u64);
impl_access!(isize);
impl_access!(usize);
impl_access!(
    &'static CStr,
    read_c_string,
    write_c_string,
    update_c_string,
    read_str_mib,
    write_c_string_mib,
    update_c_string_mib
);
impl_access!(
    &'static [u8],
    read_c_string,
    write_str_cstr,
    update_str_cstr,
    read_str_mib,
    write_str_mib,
    update_str_mib;
    CStr::to_bytes_with_nul
);

// Keep these paths qualified so rustc links jemalloc before static libc
// in musl test builds.
fn bool_to_c(value: bool) -> tikv_jemalloc_sys::c_bool {
    tikv_jemalloc_sys::c_bool::from(value)
}

fn bool_from_c(value: tikv_jemalloc_sys::c_bool) -> bool {
    value != bool_to_c(false)
}

impl Access<bool> for Name<'_, bool> {
    fn read(&self) -> Result<bool> {
        unsafe { raw::read_cstr(self.name) }.map(bool_from_c)
    }

    fn write(&self, value: bool) -> Result<()> {
        unsafe { raw::write_cstr(self.name, bool_to_c(value)) }
    }

    fn update(&self, value: bool) -> Result<bool> {
        unsafe { raw::update_cstr(self.name, bool_to_c(value)) }
            .map(bool_from_c)
    }
}

impl Access<bool> for MibRef<'_, bool> {
    fn read(&self) -> Result<bool> {
        unsafe { raw::read_mib(self.indices) }.map(bool_from_c)
    }

    fn write(&self, value: bool) -> Result<()> {
        unsafe { raw::write_mib(self.indices, bool_to_c(value)) }
    }

    fn update(&self, value: bool) -> Result<bool> {
        unsafe { raw::update_mib(self.indices, bool_to_c(value)) }
            .map(bool_from_c)
    }
}

fn decode_ctl_str(value: &CStr) -> Result<&str> {
    str::from_utf8(value.to_bytes_with_nul()).map_err(|_| Error::INVALID_UTF8)
}

impl Access<&'static str> for Name<'_, &'static str> {
    fn read(&self) -> Result<&'static str> {
        let value = unsafe { raw::read_c_string(self.name)? };
        decode_ctl_str(value)
    }

    fn write(&self, value: &'static str) -> Result<()> {
        unsafe { raw::write_str_cstr(self.name, value.as_bytes()) }
    }

    fn update(&self, value: &'static str) -> Result<&'static str> {
        let value =
            unsafe { raw::update_str_cstr(self.name, value.as_bytes())? };
        decode_ctl_str(value)
    }
}

impl Access<&'static str> for MibRef<'_, &'static str> {
    fn read(&self) -> Result<&'static str> {
        let value = unsafe { raw::read_str_mib(self.indices)? };
        decode_ctl_str(value)
    }

    fn write(&self, value: &'static str) -> Result<()> {
        unsafe { raw::write_str_mib(self.indices, value.as_bytes()) }
    }

    fn update(&self, value: &'static str) -> Result<&'static str> {
        let value =
            unsafe { raw::update_str_mib(self.indices, value.as_bytes())? };
        decode_ctl_str(value)
    }
}

#[cfg(test)]
mod tests {
    use super::{decode_ctl_str, raw, Access, CStr, Mib, Name, MAX_MIB_DEPTH};
    use crate::error::cvt;

    #[test]
    fn built_in_mibs_store_only_their_indices() {
        use crate::{config::malloc_conf_mib, epoch_mib, mem::size_of};

        assert_eq!(size_of::<epoch_mib>(), size_of::<[usize; 1]>());
        assert_eq!(size_of::<malloc_conf_mib>(), size_of::<[usize; 2]>());
    }

    #[cfg(feature = "use_std")]
    #[test]
    fn name_formatting_omits_nul_and_escapes_invalid_utf8() {
        let name = crate::epoch::name();
        assert_eq!(std::format!("{name}"), "epoch");
        assert_eq!(name.as_bytes(), b"epoch\0");

        // Exercise formatting alone, without looking up a real key.
        let name = Name::<()> {
            name: CStr::from_bytes_with_nul(b"test\xff\0").unwrap(),
            value: core::marker::PhantomData,
        };
        assert_eq!(std::format!("{name}"), r"test\xff");
        assert_eq!(std::format!("{name:?}"), r"test\xff");
        assert_eq!(name.as_bytes(), b"test\xff\0");
    }

    #[test]
    fn string_decoding_preserves_nul_and_rejects_invalid_utf8() {
        let valid = CStr::from_bytes_with_nul(b"value\0").unwrap();
        assert_eq!(decode_ctl_str(valid).unwrap(), "value\0");

        let invalid = CStr::from_bytes_with_nul(b"\xff\0").unwrap();
        assert_eq!(decode_ctl_str(invalid).map(|_| ()), cvt(libc::EILSEQ));
    }

    #[test]
    fn u32_r() {
        let name =
            unsafe { Name::<u32>::new(b"arenas.bin.0.nregs\0") }.unwrap();
        let mib = name.mib().unwrap();
        assert_eq!(mib.read().unwrap(), name.read().unwrap());
    }

    #[test]
    fn size_t_r() {
        let name =
            unsafe { Name::<libc::size_t>::new(b"arenas.lextent.0.size\0") }
                .unwrap();
        let mib = name.mib().unwrap();
        assert_eq!(mib.read().unwrap(), name.read().unwrap());
    }

    #[test]
    fn ssize_t_rw() {
        let name =
            unsafe { Name::<libc::ssize_t>::new(b"arenas.dirty_decay_ms\0") }
                .unwrap();
        let value = name.read().unwrap();
        name.write(value).unwrap();
        let mib = name.mib().unwrap();
        assert_eq!(mib.read().unwrap(), value);
    }

    fn check_string_updates<T: Copy + PartialEq + core::fmt::Debug>(
        name: &impl Access<T>,
        mib: &impl Access<T>,
        original: T,
        alternate: T,
    ) {
        let by_name = name.update(alternate);
        let read_by_mib = mib.read();
        let by_mib = mib.update(original);
        let read_by_name = name.read();
        // Restore the setting before checking the results.
        name.write(original).unwrap();

        assert_eq!(by_name.unwrap(), alternate);
        assert_eq!(read_by_mib.unwrap(), alternate);
        assert_eq!(by_mib.unwrap(), original);
        assert_eq!(read_by_name.unwrap(), original);
    }

    #[test]
    fn str_rw() {
        let name =
            unsafe { Name::<&'static [u8]>::new(b"arena.0.dss\0") }.unwrap();
        let dss = name.read().unwrap();
        let mib = name.mib().unwrap();
        assert_eq!(mib.read().unwrap(), dss);

        let alternate: &[u8] = if dss == b"disabled\0" {
            b"secondary\0"
        } else {
            b"disabled\0"
        };
        let alternate = match name.write(alternate) {
            Ok(()) => alternate,
            // jemalloc rejects other DSS modes on some targets.
            Err(err)
                if dss == b"disabled\0" && Err(err) == cvt(libc::EFAULT) =>
            {
                dss
            }
            Err(err) => panic!("cannot change DSS precedence: {}", err),
        };
        let written = mib.read();
        mib.write(dss).unwrap();
        assert_eq!(written.unwrap(), alternate);
        check_string_updates(&name, &mib, dss, alternate);

        let key = CStr::from_bytes_with_nul(b"arena.0.dss\0").unwrap();
        let name = unsafe { Name::<&'static CStr>::from_c_str(key) }.unwrap();
        let mib = name.mib().unwrap();
        let value = name.read().unwrap();
        assert_eq!(value.to_bytes_with_nul(), dss);
        assert_eq!(unsafe { raw::read_str(name.as_bytes()) }.unwrap(), value);
        assert_eq!(
            unsafe { raw::update_str(name.as_bytes(), dss) }.unwrap(),
            value
        );
        assert_eq!(mib.read().unwrap(), value);
        name.write(value).unwrap();
        mib.write(value).unwrap();
        let alternate = CStr::from_bytes_with_nul(alternate).unwrap();
        check_string_updates(&name, &mib, value, alternate);
    }

    #[test]
    fn string_write_and_update_errors() {
        let name = crate::version::name();
        let mib = name.mib().unwrap();
        for (value, errno) in [
            ("", libc::EINVAL),
            ("unterminated", libc::EINVAL),
            ("terminated\0", libc::EPERM),
            ("terminated\0suffix\0", libc::EPERM),
        ] {
            assert_eq!(name.write(value), cvt(errno));
            assert_eq!(mib.write(value), cvt(errno));
            assert_eq!(name.update(value).map(|_| ()), cvt(errno));
            assert_eq!(mib.update(value).map(|_| ()), cvt(errno));
        }
    }

    #[test]
    fn mib_exposes_only_written_indices() {
        let name = crate::arenas::narenas::name();
        let mib = name.mib().unwrap();
        let mut expected = [0; 2];
        raw::name_to_mib(name.as_bytes(), &mut expected).unwrap();
        assert_eq!(mib.as_ref(), &expected);
        assert_eq!(mib[1], expected[1]);

        // SAFETY: The indices were obtained above for the same key and type.
        let restored = unsafe { Mib::from_raw(&expected) }.unwrap();
        assert_eq!(mib, restored);
    }

    #[test]
    fn rejects_unsupported_raw_mib_lengths() {
        for indices in [&[][..], &[0; MAX_MIB_DEPTH + 1]] {
            // SAFETY: These lengths are rejected before constructing a MIB.
            let mib = unsafe { Mib::<usize>::from_raw(indices) };
            assert_eq!(mib.map(|_| ()), cvt(libc::EINVAL));
        }
    }

    #[cfg(feature = "stats")]
    #[test]
    fn resolves_deepest_mib() {
        // SAFETY: This stats key returns a uint64_t and rejects writes.
        let name = unsafe {
            Name::<u64>::new(b"stats.arenas.0.bins.0.mutex.num_ops\0")
        }
        .unwrap();
        let mib = name.mib().unwrap();
        assert_eq!(mib.as_ref().len(), MAX_MIB_DEPTH);
        assert_eq!(mib.read().unwrap(), name.read().unwrap());
    }
}
