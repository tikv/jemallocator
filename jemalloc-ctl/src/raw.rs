//! Raw access to the `mallctl` API.
//!
//! Operations that take a name reject negative index components.

use crate::error::{cvt, Result};
use crate::{
    mem::{self, MaybeUninit},
    ptr, slice,
};
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
    unsafe {
        validate_name(name)?;

        let mut len = mib.len();
        cvt(tikv_jemalloc_sys::mallctlnametomib(
            name as *const _ as *const c_char,
            mib.as_mut_ptr(),
            &mut len,
        ))?;
        assert_eq!(mib.len(), len);
        Ok(())
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
    validate_name(name)?;

    let mut value = MaybeUninit::<T>::uninit();
    let mut len = mem::size_of::<T>();
    cvt(tikv_jemalloc_sys::mallctl(
        name as *const _ as *const c_char,
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
/// and writes it `value`
///
/// # Safety
///
/// `T` and `value` must satisfy the key's type and value requirements,
/// including any requirements for pointers that jemalloc reads or retains.
pub unsafe fn write<T>(name: &[u8], mut value: T) -> Result<()> {
    validate_name(name)?;

    cvt(tikv_jemalloc_sys::mallctl(
        name as *const _ as *const c_char,
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
pub unsafe fn update<T>(name: &[u8], mut value: T) -> Result<T> {
    validate_name(name)?;

    let mut old_len = mem::size_of::<T>();
    let mut old_value = MaybeUninit::<T>::uninit();
    cvt(tikv_jemalloc_sys::mallctl(
        name as *const _ as *const c_char,
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
/// a pointer to an immutable NUL-terminated string valid for the entire
/// program.
pub unsafe fn read_str_mib(mib: &[usize]) -> Result<&'static [u8]> {
    let ptr: *const c_char = read_mib(mib)?;
    Ok(ptr2str(ptr))
}

/// Uses the MIB `mib` as key to the _MALLCTL NAMESPACE_ and writes its `value`.
///
/// The [`name_to_mib`] API translates a string of the key (e.g. `arenas.nbins`)
/// to a `mib` (Management Information Base).
///
/// # Safety
///
/// `mib` must be valid for the running jemalloc instance. If the key
/// supports writing, it must take a pointer to an immutable NUL-terminated
/// string.
///
/// # Errors
///
/// Returns `EINVAL` if `value` lacks a trailing NUL.
pub unsafe fn write_str_mib(
    mib: &[usize],
    value: &'static [u8],
) -> Result<()> {
    write_mib(mib, str_ptr(value)?)
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
/// Returns `EINVAL` if `value` lacks a trailing NUL.
pub unsafe fn update_str_mib(
    mib: &[usize],
    value: &'static [u8],
) -> Result<&'static [u8]> {
    let ptr = update_mib(mib, str_ptr(value)?)?;
    Ok(ptr2str(ptr))
}

/// Uses the null-terminated string `name` as key to the _MALLCTL NAMESPACE_ and
/// reads its value.
///
/// # Safety
///
/// The requirements of [`read()`] apply. On success, the key must return
/// a pointer to an immutable NUL-terminated string valid for the entire
/// program.
pub unsafe fn read_str(name: &[u8]) -> Result<&'static [u8]> {
    let ptr: *const c_char = read(name)?;
    Ok(ptr2str(ptr))
}

/// Uses the null-terminated string `name` as key to the _MALLCTL NAMESPACE_ and
/// writes its `value`.
///
/// # Safety
///
/// If the key supports writing, it must take a pointer to an immutable
/// NUL-terminated string.
///
/// # Errors
///
/// Returns `EINVAL` if `value` lacks a trailing NUL.
pub unsafe fn write_str(name: &[u8], value: &'static [u8]) -> Result<()> {
    write(name, str_ptr(value)?)
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
/// Returns `EINVAL` if `value` lacks a trailing NUL.
pub unsafe fn update_str(
    name: &[u8],
    value: &'static [u8],
) -> Result<&'static [u8]> {
    let ptr = update(name, str_ptr(value)?)?;
    Ok(ptr2str(ptr))
}

fn str_ptr(value: &'static [u8]) -> Result<*const c_char> {
    if value.last() != Some(&0) {
        cvt(libc::EINVAL)?;
    }
    Ok(value.as_ptr().cast())
}

/// Borrows a NUL-terminated character string, including the NUL byte.
///
/// # Panics
/// If `ptr.is_null()`.
/// # Safety
/// `ptr` must point to an immutable NUL-terminated string valid for the
/// entire program.
unsafe fn ptr2str(ptr: *const c_char) -> &'static [u8] {
    assert!(!ptr.is_null(), "attempt to borrow a null string pointer");
    let len = libc::strlen(ptr);
    slice::from_raw_parts(ptr as *const u8, len + 1)
}

fn validate_name(name: &[u8]) -> Result<()> {
    assert!(!name.is_empty(), "empty byte string");
    assert_eq!(
        *name.last().unwrap(),
        b'\0',
        "non-null terminated byte string"
    );

    // jemalloc's index parser accepts a leading sign after C whitespace.
    // Reject negative indices before it can perform a signed negation.
    let name = name.split(|&byte| byte == 0).next().unwrap();
    for component in name.split(|&byte| byte == b'.') {
        let first = component
            .iter()
            .copied()
            .find(|&byte| !matches!(byte, b' ' | b'\t'..=b'\r'));
        if first == Some(b'-') {
            return cvt(libc::EINVAL);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_ptr2str() {
        unsafe {
            //{ // This is undefined behavior:
            //    let cstr = b"";
            //    let rstr = ptr2str(cstr as *const _ as *const c_char);
            //    assert!(rstr.is_err());
            // }
            {
                let cstr = b"\0";
                let rstr = ptr2str(cstr as *const _ as *const c_char);
                assert_eq!(rstr.len(), 1);
                assert_eq!(rstr, b"\0");
            }
            {
                let cstr = b"foo  baaar\0";
                let rstr = ptr2str(cstr as *const _ as *const c_char);
                assert_eq!(rstr.len(), b"foo  baaar\0".len());
                assert_eq!(rstr, b"foo  baaar\0");
            }
        }
    }
}
