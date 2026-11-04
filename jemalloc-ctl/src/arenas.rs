//! Arena operations.

option! {
    narenas narenas_mib[ str: b"arenas.narenas\0", non_str: 2 ] => libc::c_uint |
    ops: r |
    test: narenas_read_test |
    docs:
    /// Current limit on the number of arenas.
    ///
    /// # Examples
    ///
    /// ```
    /// #
    /// # #[global_allocator]
    /// # static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
    /// #
    /// # fn main() {
    /// use tikv_jemalloc_ctl::arenas;
    /// println!("number of arenas: {}", arenas::narenas::read().unwrap());
    ///
    /// let arenas_mib = arenas::narenas::mib().unwrap();
    /// println!("number of arenas: {}", arenas_mib.read().unwrap());
    /// # }
    /// ```
    mib_docs: /// See [`narenas`].
}

/// Arena index that selects all arenas in
/// `arena.<i>.{initialized,purge,decay,dss}` and `stats.arenas.<i>.*`
/// keys. Re-exported from `tikv_jemalloc_sys`.
pub use tikv_jemalloc_sys::MALLCTL_ARENAS_ALL;

/// Purges unused dirty pages of all arenas.
///
/// Equivalent to `mallctl("arena.<MALLCTL_ARENAS_ALL>.purge", NULL, NULL, NULL, 0)`
/// in C.
///
/// # Examples
///
/// ```
/// # #[global_allocator]
/// # static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
/// #
/// # fn main() {
/// use tikv_jemalloc_ctl::arenas;
/// arenas::purge_all().unwrap();
/// # }
/// ```
pub fn purge_all() -> crate::error::Result<()> {
    let mut mib = [0; 3];
    crate::raw::name_to_mib(b"arena.0.purge\0", &mut mib)?;
    mib[1] = MALLCTL_ARENAS_ALL;
    unsafe {
        crate::error::cvt(tikv_jemalloc_sys::mallctlbymib(
            mib.as_ptr(),
            mib.len(),
            crate::ptr::null_mut(),
            crate::ptr::null_mut(),
            crate::ptr::null_mut(),
            0,
        ))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn purge_all_test() {
        // Create some dirty pages, then purge them.
        unsafe {
            let p = tikv_jemalloc_sys::malloc(16 * 1024 * 1024);
            assert!(!p.is_null());
            tikv_jemalloc_sys::free(p);
        }
        super::purge_all().unwrap();
    }
}
