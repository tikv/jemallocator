//! `jemalloc`'s profiling utils.

use crate::ffi::CStr;

option! {
    dump[ str: b"prof.dump\0", str: 2 ] => &'static CStr |
    ops: w |
    docs:
    /// Dump a memory profile to the specified file, or if NULL is specified,
    /// to a file according to the pattern <prefix>.<pid>.<seq>.m<mseq>.heap,
    /// where <prefix> is controlled by the opt.prof_prefix and prof.prefix options.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[global_allocator]
    /// # static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
    /// #
    /// # fn main() {
    /// use tikv_jemalloc_ctl::prof;
    /// let dump = prof::dump::mib().unwrap();
    /// prof.write("prof.heap").unwrap();
    /// # }
    /// ```
    mib_docs: /// See [`dump`].
}

option! {
    prefix[ str: b"prof.prefix\0", str: 2 ] => &'static CStr |
    ops: w |
    docs:
    /// Set the filename prefix for profile dumps. See opt.prof_prefix for the default setting.
    ///
    /// This can be useful to differentiate profile dumps such as from forked processes.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[global_allocator]
    /// # static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
    /// #
    /// # fn main() {
    /// use tikv_jemalloc_ctl::prof;
    /// let prefix = prof::prefix::mib().unwrap();
    /// prefix.write("my_prefix").unwrap();
    /// # }
    /// ```
    mib_docs: /// See [`prefix`].
}
