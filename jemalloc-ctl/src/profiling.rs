//! `jemalloc`'s run-time configuration for profiling-specific settings.
//!
//! These settings are controlled by the `MALLOC_CONF` environment variable.

option! {
    lg_prof_interval[ str: b"opt.lg_prof_interval\0", non_str: 2 ] => libc::ssize_t |
    ops: r |
    docs:
    /// Average interval (log base 2) between memory profile dumps, as measured in bytes of
    /// allocation activity.
    ///
    /// The actual interval between dumps may be sporadic because
    /// decentralized allocation counters are used to avoid synchronization bottlenecks.
    ///
    /// Profiles are dumped to files named according to the pattern
    /// \<prefix\>.\<pid\>.\<seq\>.i\<iseq\>.heap, where \<prefix\> is controlled by the
    /// opt.prof_prefix and prof.prefix options. By default, interval-triggered profile dumping is
    /// disabled (encoded as -1).
    ///
    /// # Examples
    ///
    /// ```
    /// # #[global_allocator]
    /// # static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
    /// #
    /// # fn main() {
    /// # #[cfg(not(windows))] {
    /// #
    /// use tikv_jemalloc_ctl::profiling;
    /// let lg_prof_interval = profiling::lg_prof_interval::read().unwrap();
    /// println!("average interval between memory profile dumps: {}", lg_prof_interval);
    /// #
    /// # } // #[cfg(..)]
    /// # }
    /// ```
    mib_docs: /// See [`lg_prof_interval`].
}

option! {
    lg_prof_sample[ str: b"opt.lg_prof_sample\0", non_str: 2 ] => libc::size_t |
    ops: r |
    docs:
    /// Average interval (log base 2) between allocation samples, as measured in bytes of
    /// allocation activity. Increasing the sampling interval decreases profile fidelity, but also
    /// decreases the computational overhead.
    ///
    /// The default sample interval is 512 KiB (2^19 B).
    ///
    /// # Examples
    ///
    /// ```
    /// # #[global_allocator]
    /// # static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
    /// #
    /// # fn main() {
    /// # #[cfg(not(windows))] {
    /// #
    /// use tikv_jemalloc_ctl::profiling;
    /// let lg_prof_sample = profiling::lg_prof_sample::read().unwrap();
    /// println!("average interval between allocation samples: {}", lg_prof_sample);
    /// #
    /// # } // #[cfg(..)]
    /// # }
    /// ```
    mib_docs: /// See [`lg_prof_sample`].
}

option! {
    prof_final[ str: b"opt.prof_final\0", non_str: 2 ] => bool |
    ops: r |
    docs:
    /// Use an atexit(3) function to dump final memory usage to a file named according to the
    /// pattern \<prefix\>.\<pid\>.\<seq\>.f.heap, where \<prefix\> is controlled by the opt.prof_prefix
    /// and prof.prefix options.
    ///
    /// Note that atexit() may allocate memory during application initialization and then deadlock
    /// internally when jemalloc in turn calls `atexit()`, so this option is not universally usable
    /// (though the application can register its own `atexit()` function with equivalent
    /// functionality).
    ///
    /// This option is disabled by default.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[global_allocator]
    /// # static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
    /// #
    /// # fn main() {
    /// # #[cfg(not(windows))] {
    /// #
    /// use tikv_jemalloc_ctl::profiling;
    /// let prof_final = profiling::prof_final::read().unwrap();
    /// println!("dump final memory usage to file: {}", prof_final);
    /// #
    /// # } // #[cfg(..)]
    /// # }
    /// ```
    mib_docs: /// See [`prof_final`].
}

option! {
    prof[ str: b"opt.prof\0", non_str: 2 ] => bool |
    ops: r |
    docs:
    /// Memory profiling enabled/disabled.
    ///
    /// If enabled, profile memory allocation activity.
    ///
    /// See the `opt.prof_active` option for on-the-fly activation/deactivation.
    ///
    /// See the `opt.lg_prof_sample` option for probabilistic sampling control.
    ///
    /// See the `opt.prof_accum` option for control of cumulative sample reporting.
    ///
    /// See the `opt.lg_prof_interval` option for information on interval-triggered profile
    /// dumping, the `opt.prof_gdump` option for information on high-water-triggered profile
    /// dumping, and the `opt.prof_final` option for final profile dumping.
    ///
    /// Profile output is compatible with the jeprof command, which is based on the pprof that is
    /// developed as part of the gperftools package. See `HEAP PROFILE FORMAT` for heap profile
    /// format documentation.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[global_allocator]
    /// # static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
    /// #
    /// # fn main() {
    /// # #[cfg(not(windows))] {
    /// #
    /// use tikv_jemalloc_ctl::profiling;
    /// let prof = profiling::prof::read().unwrap();
    /// println!("is memory profiling enabled: {}", prof);
    /// #
    /// # } // #[cfg(..)]
    /// # }
    /// ```
    mib_docs: /// See [`prof`].
}

option! {
    prof_leak[ str: b"opt.prof_leak\0", non_str: 2 ] => bool |
    ops: r |
    docs:
    /// Leak reporting enabled/disabled.
    ///
    /// If enabled, use an `atexit(3)` function to report memory leaks detected by allocation
    /// sampling.
    ///
    /// See the opt.prof option for information on analyzing heap profile output.
    ///
    /// Works only when combined with `opt.prof_final`, otherwise does nothing.
    ///
    /// This option is disabled by default.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[global_allocator]
    /// # static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
    /// #
    /// # fn main() {
    /// # #[cfg(not(windows))] {
    /// #
    /// use tikv_jemalloc_ctl::profiling;
    /// let prof_leak = profiling::prof_leak::read().unwrap();
    /// println!("is leak reporting enabled: {}", prof_leak);
    /// #
    /// # } // #[cfg(..)]
    /// # }
    /// ```
    mib_docs: /// See [`prof_leak`].
}
