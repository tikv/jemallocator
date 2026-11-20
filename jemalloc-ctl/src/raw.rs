//! Raw access to the `mallctl` API.
//!
//! Operations that take a name return `EINVAL` for malformed C strings
//! or negative index components.
//!
//! String reads and updates borrow immutable, program-long C strings and
//! return `EINVAL` for null output pointers. `arena.<i>.name` requires a
//! caller-provided output buffer. Keys such as `thread.prof.name` and
//! `opt.malloc_conf.env_var` return pointers into storage that can change
//! or be released. Handling those pointers requires the key's own lifetime
//! and synchronization rules.

use crate::error::{cvt, Error, Result};
use crate::{
    mem::{self, MaybeUninit},
    ptr,
};
use core::ffi::CStr;
use libc::c_char;

/// Translates `name` to a `mib` (Management Information Base)
///
/// `mib`s are used to avoid repeated name lookups for applications that
/// repeatedly query the same portion of `jemalloc`s `mallctl` namespace.
///
/// On success, `mib` contains an array of integers. It is possible to pass
/// `mib` with a length smaller than the number of period-separated name
/// components. This results in a partial MIB that can be used as the basis for
/// constructing a complete MIB.
///
/// Returns `EINVAL` if `mib` is longer than the complete MIB. The lookup may
/// have already written the indices at the start of `mib`.
///
/// For name components that are integers (e.g. the `2` in `arenas.bin.2.size`),
/// the corresponding MIB component will always be that integer. Therefore, it
/// is legitimate to construct code like the following:
///
/// ```
/// #[global_allocator]
/// static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
///
/// fn main() {
///     use tikv_jemalloc_ctl::raw;
///     use libc::{c_uint, c_char};
///     unsafe {
///         let mut mib = [0; 4];
///         let nbins: c_uint = raw::read(b"arenas.nbins\0").unwrap();
///         raw::name_to_mib(b"arenas.bin.0.size\0", &mut mib).unwrap();
///         for i in 0..4 {
///             mib[2] = i;
///             let bin_size: usize = raw::read_mib(&mut mib).unwrap();
///             println!("arena bin {} has size {}", i, bin_size);
///         }
///     }
/// }
/// ```
pub fn name_to_mib(name: &[u8], mib: &mut [usize]) -> Result<()> {
    let len = name_to_mib_cstr(parse_name(name)?, mib)?;
    if len != mib.len() {
        return Err(Error::INVALID_ARGUMENT);
    }
    Ok(())
}

pub(crate) fn name_to_mib_cstr(
    name: &CStr,
    mib: &mut [usize],
) -> Result<usize> {
    validate_indices(name)?;
    unsafe {
        let mut len = mib.len();
        cvt(tikv_jemalloc_sys::mallctlnametomib(
            name.as_ptr(),
            mib.as_mut_ptr(),
            &mut len,
        ))?;
        Ok(len)
    }
}

/// Uses the MIB `mib` as key to the _MALLCTL NAMESPACE_ and reads its value.
///
/// The [`name_to_mib`] API translates a string of the key (e.g. `arenas.nbins`)
/// to a `mib` (Management Information Base).
///
/// # Safety
///
/// `mib` must be valid for the running jemalloc instance. The key must
/// treat `oldp` as a pure output buffer, whose contents are initially
/// uninitialized. On success, it must fully initialize a valid `T`.
/// Keys such as `arena.<i>.name` that require a caller-provided pointer
/// in `oldp` need a direct `mallctlbymib` call instead.
pub unsafe fn read_mib<T: Copy>(mib: &[usize]) -> Result<T> {
    let mut value = MaybeUninit::<T>::uninit();
    let mut len = mem::size_of::<T>();
    cvt(tikv_jemalloc_sys::mallctlbymib(
        mib.as_ptr(),
        mib.len(),
        value.as_mut_ptr() as *mut _,
        &mut len,
        ptr::null_mut(),
        0,
    ))?;
    assert_eq!(len, mem::size_of::<T>());
    Ok(value.assume_init())
}

/// Uses the null-terminated string `name` as key to the _MALLCTL NAMESPACE_ and
/// reads its value.
///
/// # Safety
///
/// The key must treat `oldp` as a pure output buffer, whose contents are
/// initially uninitialized. On success, it must fully initialize a valid
/// `T`. Keys such as `arena.<i>.name` that require a caller-provided
/// pointer in `oldp` need a direct `mallctl` call instead.
pub unsafe fn read<T: Copy>(name: &[u8]) -> Result<T> {
    read_cstr(parse_name(name)?)
}

pub(crate) unsafe fn read_cstr<T: Copy>(name: &CStr) -> Result<T> {
    validate_indices(name)?;

    let mut value = MaybeUninit::<T>::uninit();
    let mut len = mem::size_of::<T>();
    cvt(tikv_jemalloc_sys::mallctl(
        name.as_ptr(),
        value.as_mut_ptr() as *mut _,
        &mut len,
        ptr::null_mut(),
        0,
    ))?;
    assert_eq!(len, mem::size_of::<T>());
    Ok(value.assume_init())
}

/// Uses the MIB `mib` as key to the _MALLCTL NAMESPACE_ and writes its `value`.
///
/// The [`name_to_mib`] API translates a string of the key (e.g. `arenas.nbins`)
/// to a `mib` (Management Information Base).
///
/// # Safety
///
/// `mib` must be valid for the running jemalloc instance. `T` and `value`
/// must satisfy the key's type and value requirements, including any
/// requirements for pointers that jemalloc reads or retains.
pub unsafe fn write_mib<T>(mib: &[usize], mut value: T) -> Result<()> {
    cvt(tikv_jemalloc_sys::mallctlbymib(
        mib.as_ptr(),
        mib.len(),
        ptr::null_mut(),
        ptr::null_mut(),
        &mut value as *mut _ as *mut _,
        mem::size_of::<T>(),
    ))
}

/// Uses the null-terminated string `name` as the key to the _MALLCTL NAMESPACE_
/// and writes its `value`.
///
/// # Safety
///
/// `T` and `value` must satisfy the key's type and value requirements,
/// including any requirements for pointers that jemalloc reads or retains.
pub unsafe fn write<T>(name: &[u8], value: T) -> Result<()> {
    write_cstr(parse_name(name)?, value)
}

pub(crate) unsafe fn write_cstr<T>(name: &CStr, mut value: T) -> Result<()> {
    validate_indices(name)?;

    cvt(tikv_jemalloc_sys::mallctl(
        name.as_ptr(),
        ptr::null_mut(),
        ptr::null_mut(),
        &mut value as *mut _ as *mut _,
        mem::size_of::<T>(),
    ))
}

/// Uses the MIB `mib` as key to the _MALLCTL NAMESPACE_ and writes its `value`
/// returning the value reported by the key.
///
/// The [`name_to_mib`] API translates a string of the key (e.g. `arenas.nbins`)
/// to a `mib` (Management Information Base).
///
/// # Safety
///
/// The requirements of [`read_mib`] and [`write_mib`] both apply.
pub unsafe fn update_mib<T: Copy>(mib: &[usize], mut value: T) -> Result<T> {
    let mut old_len = mem::size_of::<T>();
    let mut old_value = MaybeUninit::<T>::uninit();
    cvt(tikv_jemalloc_sys::mallctlbymib(
        mib.as_ptr(),
        mib.len(),
        old_value.as_mut_ptr() as *mut _,
        &mut old_len,
        &mut value as *mut _ as *mut _,
        mem::size_of::<T>(),
    ))?;
    assert_eq!(old_len, mem::size_of::<T>());
    Ok(old_value.assume_init())
}

/// Uses the null-terminated string `name` as key to the _MALLCTL NAMESPACE_ and
/// writes its `value` and returns the value reported by the key.
///
/// # Safety
///
/// The requirements of [`read()`] and [`write()`] both apply.
pub unsafe fn update<T>(name: &[u8], value: T) -> Result<T> {
    update_cstr(parse_name(name)?, value)
}

pub(crate) unsafe fn update_cstr<T>(name: &CStr, mut value: T) -> Result<T> {
    validate_indices(name)?;

    let mut old_len = mem::size_of::<T>();
    let mut old_value = MaybeUninit::<T>::uninit();
    cvt(tikv_jemalloc_sys::mallctl(
        name.as_ptr(),
        old_value.as_mut_ptr() as *mut _,
        &mut old_len,
        &mut value as *mut _ as *mut _,
        mem::size_of::<T>(),
    ))?;
    assert_eq!(old_len, mem::size_of::<T>());
    Ok(old_value.assume_init())
}

/// Uses the MIB `mib` as key to the _MALLCTL NAMESPACE_ and reads its value.
///
/// The [`name_to_mib`] API translates a string of the key (e.g. `arenas.nbins`)
/// to a `mib` (Management Information Base).
///
/// # Safety
///
/// The requirements of [`read_mib`] apply. On success, the key must return
/// a null pointer or a pointer to an immutable NUL-terminated string valid
/// for the entire program.
pub unsafe fn read_str_mib(mib: &[usize]) -> Result<&'static CStr> {
    borrow_c_string(read_mib(mib)?)
}

/// Uses the MIB `mib` as key to the _MALLCTL NAMESPACE_ and writes its `value`.
///
/// The [`name_to_mib`] API translates a string of the key (e.g. `arenas.nbins`)
/// to a `mib` (Management Information Base).
///
/// # Safety
///
/// The requirements of [`write_mib`] apply. If the key supports writing,
/// it must take a pointer to an immutable NUL-terminated string.
///
/// # Errors
///
/// Returns `EINVAL` if `value` lacks a trailing NUL.
pub unsafe fn write_str_mib(
    mib: &[usize],
    value: &'static [u8],
) -> Result<()> {
    write_mib(mib, string_value_ptr(value)?)
}

pub(crate) unsafe fn write_c_string_mib(
    mib: &[usize],
    value: &'static CStr,
) -> Result<()> {
    write_mib(mib, value.as_ptr())
}

/// Uses the MIB `mib` as key to the _MALLCTL NAMESPACE_ and writes its `value`
/// returning the value reported by the key.
///
/// The [`name_to_mib`] API translates a string of the key (e.g. `arenas.nbins`)
/// to a `mib` (Management Information Base).
///
/// # Safety
///
/// The requirements of [`read_str_mib`] and [`write_str_mib`] both apply.
///
/// # Errors
///
/// Returns `EINVAL` if `value` lacks a trailing NUL or the result is null.
pub unsafe fn update_str_mib(
    mib: &[usize],
    value: &'static [u8],
) -> Result<&'static CStr> {
    borrow_c_string(update_mib(mib, string_value_ptr(value)?)?)
}

pub(crate) unsafe fn update_c_string_mib(
    mib: &[usize],
    value: &'static CStr,
) -> Result<&'static CStr> {
    borrow_c_string(update_mib(mib, value.as_ptr())?)
}

/// Uses the null-terminated string `name` as key to the _MALLCTL NAMESPACE_ and
/// reads its value.
///
/// # Safety
///
/// The requirements of [`read()`] apply. On success, the key must return
/// a null pointer or a pointer to an immutable NUL-terminated string valid
/// for the entire program.
pub unsafe fn read_str(name: &[u8]) -> Result<&'static CStr> {
    read_c_string(parse_name(name)?)
}

pub(crate) unsafe fn read_c_string(name: &CStr) -> Result<&'static CStr> {
    borrow_c_string(read_cstr(name)?)
}

/// Uses the null-terminated string `name` as key to the _MALLCTL NAMESPACE_ and
/// writes its `value`.
///
/// # Safety
///
/// The requirements of [`write()`] apply. If the key supports writing,
/// it must take a pointer to an immutable NUL-terminated string.
///
/// # Errors
///
/// Returns `EINVAL` if `value` lacks a trailing NUL.
pub unsafe fn write_str(name: &[u8], value: &'static [u8]) -> Result<()> {
    write_str_cstr(parse_name(name)?, value)
}

pub(crate) unsafe fn write_str_cstr(
    name: &CStr,
    value: &'static [u8],
) -> Result<()> {
    write_cstr(name, string_value_ptr(value)?)
}

pub(crate) unsafe fn write_c_string(
    name: &CStr,
    value: &'static CStr,
) -> Result<()> {
    write_cstr(name, value.as_ptr())
}

/// Uses the null-terminated string `name` as key to the _MALLCTL NAMESPACE_ and
/// writes its `value` and returns the value reported by the key.
///
/// # Safety
///
/// The requirements of [`read_str`] and [`write_str`] both apply.
///
/// # Errors
///
/// Returns `EINVAL` if `value` lacks a trailing NUL or the result is null.
pub unsafe fn update_str(
    name: &[u8],
    value: &'static [u8],
) -> Result<&'static CStr> {
    update_str_cstr(parse_name(name)?, value)
}

pub(crate) unsafe fn update_str_cstr(
    name: &CStr,
    value: &'static [u8],
) -> Result<&'static CStr> {
    borrow_c_string(update_cstr(name, string_value_ptr(value)?)?)
}

pub(crate) unsafe fn update_c_string(
    name: &CStr,
    value: &'static CStr,
) -> Result<&'static CStr> {
    borrow_c_string(update_cstr(name, value.as_ptr())?)
}

fn string_value_ptr(value: &'static [u8]) -> Result<*const c_char> {
    // The trailing NUL guarantees termination. C handles any earlier NUL.
    if value.last() != Some(&0) {
        return Err(Error::INVALID_ARGUMENT);
    }
    Ok(value.as_ptr().cast())
}

/// # Safety
/// A non-null `ptr` must point to an immutable NUL-terminated string valid
/// for the entire program.
unsafe fn borrow_c_string(ptr: *const c_char) -> Result<&'static CStr> {
    if ptr.is_null() {
        return Err(Error::INVALID_ARGUMENT);
    }
    Ok(CStr::from_ptr(ptr))
}

fn parse_name(name: &[u8]) -> Result<&CStr> {
    CStr::from_bytes_with_nul(name).map_err(|_| Error::INVALID_ARGUMENT)
}

fn validate_indices(name: &CStr) -> Result<()> {
    // ctl_lookup checks the index range after malloc_strtoumax returns.
    // The parser accepts '-' after C whitespace and can negate INTMAX_MIN,
    // causing signed overflow before the range check.
    for component in name.to_bytes().split(|&byte| byte == b'.') {
        let first = component
            .iter()
            .copied()
            .find(|&byte| !matches!(byte, b' ' | b'\t'..=b'\r'));
        if first == Some(b'-') {
            return Err(Error::INVALID_ARGUMENT);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_to_mib_rejects_oversized_buffers() {
        let mut mib = [0; 2];
        assert_eq!(name_to_mib(b"epoch\0", &mut mib), cvt(libc::EINVAL));
    }

    #[test]
    fn name_to_mib_accepts_partial_mibs() {
        let mut full = [0; 2];
        let mut partial = [0; 1];
        let name = b"arenas.narenas\0";
        name_to_mib(name, &mut full).unwrap();
        name_to_mib(name, &mut partial).unwrap();
        assert_eq!(partial, full[..1]);
    }

    #[test]
    fn name_to_mib_rejects_malformed_names() {
        let mut mib = [0; 1];
        for name in [&b""[..], b"version", b"version\0suffix\0"] {
            assert_eq!(name_to_mib(name, &mut mib), cvt(libc::EINVAL));
        }
    }

    #[test]
    fn name_to_mib_rejects_negative_indices() {
        let mut mib = [0; 4];
        for name in [
            &b"arenas.bin.-1.nregs\0"[..],
            &b"arenas.bin.\x0b-1.nregs\0"[..],
        ] {
            assert_eq!(name_to_mib(name, &mut mib), cvt(libc::EINVAL));
        }
    }

    #[test]
    fn borrow_c_string_includes_terminator() {
        for bytes in [&b"\0"[..], b"foo  baaar\0"] {
            let value =
                unsafe { borrow_c_string(bytes.as_ptr().cast()) }.unwrap();
            assert_eq!(value.to_bytes_with_nul(), bytes);
        }
    }

    #[test]
    fn borrow_c_string_rejects_null() {
        assert_eq!(
            unsafe { borrow_c_string(ptr::null()) },
            Err(Error::INVALID_ARGUMENT)
        );
    }
}
