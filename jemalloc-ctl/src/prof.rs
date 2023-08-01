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
    /// #[cfg(feature = "profiling")]
    /// {
    ///     use tikv_jemalloc_ctl::prof;
    ///     use std::ffi::CStr;
    ///     let dump_file_name = CStr::from_bytes_with_nul(b"dump\0").unwrap();
    ///     let dump = prof::dump::mib().unwrap();
    ///     dump.write(dump_file_name).unwrap();
    /// }
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
    /// #[cfg(feature = "profiling")]
    /// {
    ///     use tikv_jemalloc_ctl::prof;
    ///     use std::ffi::CStr;
    ///     let dump_file_name = CStr::from_bytes_with_nul(b"my_prefix\0").unwrap();
    ///     let prefix = prof::prefix::mib().unwrap();
    ///     prefix.write(dump_file_name).unwrap();
    /// }
    /// # }
    /// ```
    mib_docs: /// See [`prefix`].
}

option! {
    active[ str: b"prof.active\0", non_str: 2 ] => bool |
    ops: r, w, u |
    docs:
    /// Control whether sampling is currently active.
    ///
    /// See the `opt.prof_active` option for additional information,
    /// as well as the interrelated `thread.prof.active` mallctl.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[global_allocator]
    /// # static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
    /// #
    /// # fn main() {
    /// #[cfg(feature = "profiling")]
    /// {
    ///     use tikv_jemalloc_ctl::prof;
    ///     let active = prof::active::mib().unwrap();
    ///     active.write(true).unwrap();
    /// }
    /// # }
    /// ```
    mib_docs: /// See [`active`].
}
