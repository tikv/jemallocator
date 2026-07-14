//! `jemalloc`'s run-time configuration for profiling-specific settings.
//!
//! These settings are controlled by the `MALLOC_CONF` environment variable.
//!
//! This module also exposes on-the-fly control via [`prof_active`] and
//! [`prof_reset`].
#![cfg_attr(
    feature = "profiling_hooks",
    doc = "
With the `profiling_hooks` feature, it additionally exposes `jemalloc`'s
experimental `experimental.hooks.prof_sample`/`prof_sample_free`/
`prof_backtrace` hooks via [`set_prof_sample_hook`],
[`set_prof_sample_free_hook`], and [`set_prof_backtrace_hook`]."
)]

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
    /// use tikv_jemalloc_ctl::profiling;
    /// let lg_prof_interval = profiling::lg_prof_interval::read().unwrap();
    /// println!("average interval between memory profile dumps: {}", lg_prof_interval);
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
    /// use tikv_jemalloc_ctl::profiling;
    /// let lg_prof_sample = profiling::lg_prof_sample::read().unwrap();
    /// println!("average interval between allocation samples: {}", lg_prof_sample);
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
    /// use tikv_jemalloc_ctl::profiling;
    /// let prof_final = profiling::prof_final::read().unwrap();
    /// println!("dump final memory usage to file: {}", prof_final);
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
    /// use tikv_jemalloc_ctl::profiling;
    /// let prof = profiling::prof::read().unwrap();
    /// println!("is memory profiling enabled: {}", prof);
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
    /// use tikv_jemalloc_ctl::profiling;
    /// let prof_leak = profiling::prof_leak::read().unwrap();
    /// println!("is leak reporting enabled: {}", prof_leak);
    /// # }
    /// ```
    mib_docs: /// See [`prof_leak`].
}

option! {
    prof_active[ str: b"prof.active\0", non_str: 2 ] => bool |
    ops: r,w,u |
    docs:
    /// On-the-fly activation/deactivation of memory profiling.
    ///
    /// This is a secondary control mechanism on top of `opt.prof`, and is
    /// only effective once `opt.prof` is `true`; when it is `false`, reading
    /// [`prof_active`] always returns `false` and writing to it fails with
    /// `ENOENT`. `jemalloc` initializes [`prof_active`] to
    /// `opt.prof_active` (which itself defaults to `true`) as soon as
    /// `opt.prof` is `true`, so a build with `opt.prof` enabled samples by
    /// default unless [`prof_active`] is set to `false`, e.g. via
    /// `prof_active:false` in `MALLOC_CONF`.
    ///
    /// Note: `opt.prof_thread_active_init` is unrelated — it controls the
    /// per-thread `thread.prof.active` flag, not this global toggle.
    ///
    /// While inactive, sampling hooks installed via the `profiling_hooks`
    /// feature's hook setters remain installed but do not fire, since no
    /// allocation is ever selected for sampling.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[global_allocator]
    /// # static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
    /// #
    /// # fn main() {
    /// use tikv_jemalloc_ctl::profiling;
    /// // `false` is always accepted, even if `opt.prof` is disabled at
    /// // runtime; writing `true` additionally requires `opt.prof` to be
    /// // `true`, else it fails with `ENOENT`.
    /// let was_active = profiling::prof_active::write(false).unwrap();
    /// # let _ = was_active;
    /// # }
    /// ```
    mib_docs: /// See [`prof_active`].
}

/// Resets `jemalloc`'s heap profile sample accumulators and, going forward,
/// samples allocations at a rate of one per `2^lg_sample` bytes of
/// allocation activity.
///
/// Corresponds to `prof.reset`, which is write-only: unlike most keys in
/// this module, there is no matching `read()`/`update()`.
///
/// # Errors
///
/// Returns an error (`ENOENT`) if `opt.prof` is `false` at runtime, e.g.
/// because `MALLOC_CONF`/`JEMALLOC_SYS_WITH_MALLOC_CONF` overrode the
/// `prof:true` that the `profiling_hooks` feature bakes in (the `profiling`
/// feature alone enables `--enable-prof` at build time but does not set
/// `prof:true` in `malloc_conf`).
pub fn prof_reset(lg_sample: libc::size_t) -> crate::error::Result<()> {
    unsafe { crate::raw::write(b"prof.reset\0", lg_sample) }
}

#[cfg(feature = "profiling_hooks")]
use libc::{c_uint, c_void};

/// Signature of a hook installable via [`set_prof_sample_hook`].
///
/// `jemalloc` invokes this hook synchronously, inline on the allocating
/// thread, immediately after it decides to sample an allocation of
/// `usable_size` bytes at `ptr` (the request was for `size` bytes;
/// `usable_size` is jemalloc's usable/rounded-up size). `backtrace` points
/// to `backtrace_length`
/// `void*` frames captured by the installed [`ProfBacktraceHook`]
/// (`backtrace_length` is `0` if [`noop_prof_backtrace_hook`] is installed).
///
/// # Safety
///
/// No `jemalloc` mutex is held while this hook runs, but it does run inside
/// `jemalloc`'s `pre_reentrancy`/`post_reentrancy` bracket and may itself
/// allocate/free (including recursively sampling). It must not unwind
/// across the `extern "C"` boundary — a panic here is undefined behavior
/// below Rust 1.81, and this crate is `no_std` by default so `catch_unwind`
/// is unavailable regardless.
#[cfg(feature = "profiling_hooks")]
pub type ProfSampleHook = unsafe extern "C" fn(
    ptr: *const c_void,
    size: libc::size_t,
    backtrace: *mut *mut c_void,
    backtrace_length: c_uint,
    usable_size: libc::size_t,
);

/// Signature of a hook installable via [`set_prof_sample_free_hook`].
///
/// `jemalloc` invokes this hook synchronously, inline on the freeing
/// thread, just before it frees a previously-sampled allocation of
/// `usable_size` bytes at `ptr`. See [`ProfSampleHook`] for the applicable safety
/// contract.
#[cfg(feature = "profiling_hooks")]
pub type ProfSampleFreeHook =
    unsafe extern "C" fn(ptr: *const c_void, usable_size: libc::size_t);

/// Signature of a hook installable via [`set_prof_backtrace_hook`].
///
/// `jemalloc` invokes this hook to capture the stack trace for a sample; it
/// must write at most `max_length` frames into `backtrace` and store the
/// number of frames written through `backtrace_length`. See
/// [`ProfSampleHook`] for the applicable safety contract.
#[cfg(feature = "profiling_hooks")]
pub type ProfBacktraceHook = unsafe extern "C" fn(
    backtrace: *mut *mut c_void,
    backtrace_length: *mut c_uint,
    max_length: c_uint,
);

/// Installs, replaces, or (with `None`) uninstalls the hook `jemalloc`
/// calls after deciding to sample an allocation, returning the
/// previously-installed hook.
///
/// Corresponds to `experimental.hooks.prof_sample`.
///
/// # Errors
///
/// Returns an error (`ENOENT`) if `opt.prof` is `false` at runtime — see
/// [`prof_reset`]. Note that `opt.prof` being `true` is sufficient to
/// install a hook; [`prof_active`] need not be `true` (installing while
/// inactive is a no-op until activated).
#[cfg(feature = "profiling_hooks")]
pub fn set_prof_sample_hook(
    hook: Option<ProfSampleHook>,
) -> crate::error::Result<Option<ProfSampleHook>> {
    unsafe { crate::raw::update(b"experimental.hooks.prof_sample\0", hook) }
}

/// Installs, replaces, or (with `None`) uninstalls the hook `jemalloc`
/// calls just before freeing a previously-sampled allocation, returning the
/// previously-installed hook.
///
/// Corresponds to `experimental.hooks.prof_sample_free`. See
/// [`set_prof_sample_hook`] for the applicable error semantics.
#[cfg(feature = "profiling_hooks")]
pub fn set_prof_sample_free_hook(
    hook: Option<ProfSampleFreeHook>,
) -> crate::error::Result<Option<ProfSampleFreeHook>> {
    unsafe {
        crate::raw::update(b"experimental.hooks.prof_sample_free\0", hook)
    }
}

/// Installs or replaces the hook `jemalloc` calls to capture a sample's
/// backtrace, returning the previously-installed hook.
///
/// Corresponds to `experimental.hooks.prof_backtrace`. Unlike
/// [`set_prof_sample_hook`]/[`set_prof_sample_free_hook`], this hook cannot
/// be uninstalled (`jemalloc` rejects a `NULL` new hook with `EINVAL`) —
/// install [`noop_prof_backtrace_hook`] instead of `jemalloc`'s default
/// unwinder if backtraces aren't wanted, and keep the returned previous
/// hook only to log/inspect, not to call directly, since it may itself be a
/// previously-installed [`noop_prof_backtrace_hook`] or `jemalloc`'s
/// internal default depending on prior state.
///
/// # Errors
///
/// Returns an error (`ENOENT`) if `opt.prof` is `false` at runtime — see
/// [`prof_reset`].
#[cfg(feature = "profiling_hooks")]
pub fn set_prof_backtrace_hook(
    hook: ProfBacktraceHook,
) -> crate::error::Result<ProfBacktraceHook> {
    unsafe { crate::raw::update(b"experimental.hooks.prof_backtrace\0", hook) }
}

/// A [`ProfBacktraceHook`] that reports an empty backtrace for every
/// sample.
///
/// Installing this via [`set_prof_backtrace_hook`] disables `jemalloc`'s
/// own stack unwinding going forward: the per-allocation sampling
/// decision still happens at the configured rate (see [`lg_prof_sample`])
/// and [`set_prof_sample_hook`]/[`set_prof_sample_free_hook`] hooks still
/// fire, but with `backtrace_length` reported as `0`. Intended for
/// out-of-process samplers (e.g. an eBPF profiler) that capture their own
/// stacks and only need `jemalloc`'s sampling clock, since capturing a
/// backtrace it already unwinds itself is otherwise a per-sample cost
/// (page-aligned promotion, `tcache` bypass, and a `tdata` mutex are still
/// paid regardless of whether a backtrace is captured).
///
/// # Safety
///
/// Must only be invoked by `jemalloc` itself as an
/// `experimental.hooks.prof_backtrace` hook, which always passes a non-null
/// `backtrace_length`.
#[cfg(feature = "profiling_hooks")]
pub unsafe extern "C" fn noop_prof_backtrace_hook(
    _backtrace: *mut *mut c_void,
    backtrace_length: *mut c_uint,
    _max_length: c_uint,
) {
    *backtrace_length = 0;
}

// Heap-allocates to force samples, so this needs a real allocator (`use_std`).
#[cfg(all(test, feature = "profiling_hooks", feature = "use_std"))]
mod hook_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static SAMPLE_HOOK_CALLS: AtomicUsize = AtomicUsize::new(0);
    static SAMPLE_FREE_HOOK_CALLS: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "C" fn counting_sample_hook(
        _ptr: *const c_void,
        _size: libc::size_t,
        _backtrace: *mut *mut c_void,
        _backtrace_length: c_uint,
        _usable_size: libc::size_t,
    ) {
        SAMPLE_HOOK_CALLS.fetch_add(1, Ordering::SeqCst);
    }

    unsafe extern "C" fn counting_sample_free_hook(
        _ptr: *const c_void,
        _usable_size: libc::size_t,
    ) {
        SAMPLE_FREE_HOOK_CALLS.fetch_add(1, Ordering::SeqCst);
    }

    // Exercises the whole hook contract end-to-end against real `jemalloc`
    // ctls: activates profiling, resets the sampler to catch every
    // allocation, installs counting hooks, allocates/frees, and asserts both
    // hooks actually fired before restoring prior state.
    #[test]
    fn sample_and_sample_free_hooks_fire() {
        let was_active = prof_active::read().unwrap();
        let prev_lg_sample = lg_prof_sample::read().unwrap();
        // lg_sample: 0 => average one sample per byte, i.e. every allocation.
        // `jemalloc` only recomputes each thread's next sample distance
        // (from the new `lg_sample`) once the current, already-primed
        // distance (drawn under whatever `lg_sample` was in effect before
        // this call, e.g. the crate's default of 512 KiB) has been
        // exhausted — so the very next allocation isn't guaranteed to
        // sample yet, only allocations after that first one are.
        prof_reset(0).unwrap();
        prof_active::write(true).unwrap();

        let prev_sample =
            set_prof_sample_hook(Some(counting_sample_hook)).unwrap();
        let prev_sample_free =
            set_prof_sample_free_hook(Some(counting_sample_free_hook))
                .unwrap();

        // Warm up past any stale pre-reset sample distance: 16 MiB is many
        // times the largest plausible leftover distance from a 512 KiB mean.
        for _ in 0..16 {
            drop(Box::new([0u8; 1024 * 1024]));
        }

        let before_sample = SAMPLE_HOOK_CALLS.load(Ordering::SeqCst);
        let before_free = SAMPLE_FREE_HOOK_CALLS.load(Ordering::SeqCst);
        for _ in 0..16 {
            drop(Box::new([0u8; 4096]));
            if SAMPLE_HOOK_CALLS.load(Ordering::SeqCst) > before_sample
                && SAMPLE_FREE_HOOK_CALLS.load(Ordering::SeqCst) > before_free
            {
                break;
            }
        }

        set_prof_sample_hook(prev_sample).unwrap();
        set_prof_sample_free_hook(prev_sample_free).unwrap();
        prof_active::write(was_active).unwrap();
        prof_reset(prev_lg_sample).unwrap();

        assert!(
            SAMPLE_HOOK_CALLS.load(Ordering::SeqCst) > before_sample,
            "prof_sample hook did not fire after warm-up with lg_sample=0"
        );
        assert!(
            SAMPLE_FREE_HOOK_CALLS.load(Ordering::SeqCst) > before_free,
            "prof_sample_free hook did not fire after freeing sampled allocations"
        );
    }

    #[test]
    fn backtrace_hook_can_be_replaced_and_restored() {
        let prev = set_prof_backtrace_hook(noop_prof_backtrace_hook).unwrap();
        set_prof_backtrace_hook(prev).unwrap();
    }
}
