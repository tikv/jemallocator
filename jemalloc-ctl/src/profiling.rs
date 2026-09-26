//! `jemalloc`'s run-time configuration for profiling-specific settings.
//!
//! These settings are controlled by the `MALLOC_CONF` environment variable.
//!
//! This module also exposes on-the-fly control via [`prof_active`] and
//! [`prof_reset`], along with `jemalloc`'s experimental
//! `experimental.hooks.prof_sample`/`prof_sample_free`/`prof_backtrace` hooks
//! via [`set_prof_sample_hook`], [`set_prof_sample_free_hook`], and
//! [`set_prof_backtrace_hook`].
//!
//! # Experimental hook API
//!
//! `jemalloc` considers these hook mallctls experimental. Their names and
//! callback ABIs may change between versions without notice. Hooks are
//! process-wide, may run concurrently, and replacement does not wait for
//! in-flight calls.

use libc::{c_uint, c_void};

option! {
    lg_prof_interval lg_prof_interval_mib[ str: b"opt.lg_prof_interval\0", non_str: 2 ] => libc::ssize_t |
    ops: r |
    test: lg_prof_interval_read_test |
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
    lg_prof_sample lg_prof_sample_mib[ str: b"opt.lg_prof_sample\0", non_str: 2 ] => libc::size_t |
    ops: r |
    test: lg_prof_sample_read_test |
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
    prof_final prof_final_mib[ str: b"opt.prof_final\0", non_str: 2 ] => bool |
    ops: r |
    test: prof_final_read_test |
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
    prof prof_mib[ str: b"opt.prof\0", non_str: 2 ] => bool |
    ops: r |
    test: prof_read_test |
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
    prof_leak prof_leak_mib[ str: b"opt.prof_leak\0", non_str: 2 ] => bool |
    ops: r |
    test: prof_leak_read_test |
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
    prof_active prof_active_mib[ str: b"prof.active\0", non_str: 2 ] => bool |
    ops: r,w,u |
    test: prof_active_read_write_update_test |
    docs:
    /// On-the-fly activation/deactivation of memory profiling.
    ///
    /// This is a secondary control mechanism on top of `opt.prof`, and is
    /// only effective once `opt.prof` is `true`. When it is `false`, reading
    /// [`prof_active`] returns `false`; writing `false` is accepted as a no-op,
    /// while writing `true` fails with `ENOENT`. `jemalloc` initialises
    /// [`prof_active`] to
    /// `opt.prof_active` (which itself defaults to `true`) as soon as
    /// `opt.prof` is `true`, so a configuration with `opt.prof` enabled samples
    /// by default unless [`prof_active`] is set to `false`, e.g. via
    /// `prof_active:false` in `MALLOC_CONF`.
    ///
    /// `thread.prof.active` is a separate per-thread gate, initialised from
    /// `opt.prof_thread_active_init`; both it and this global control must be
    /// active for a thread to sample.
    ///
    /// While this control is inactive, no new allocation samples are selected,
    /// so [`ProfSampleHook`] does not fire. [`ProfSampleFreeHook`] can still
    /// fire for allocations sampled before deactivation.
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
    /// let was_active = profiling::prof_active::update(false).unwrap();
    /// profiling::prof_active::write(was_active).unwrap();
    /// # }
    /// ```
    mib_docs: /// See [`prof_active`].
}

option! {
    lg_sample lg_sample_mib[ str: b"prof.lg_sample\0", non_str: 2 ] => libc::size_t |
    ops: r |
    test: lg_sample_read_test |
    docs:
    /// Current log base 2 of the mean number of bytes between samples.
    ///
    /// Initialised from [`lg_prof_sample`] and updated by [`prof_reset`].
    mib_docs: /// See [`lg_sample`].
}

/// Resets `jemalloc`'s heap profile sample accumulators and, going forward,
/// draws sample intervals from a geometric distribution with a mean of
/// `2^new_lg_sample` bytes of allocation activity (values `>= 64` are clamped
/// to `63`). Sampling still requires `prof.active` and `thread.prof.active`.
///
/// Corresponds to `prof.reset`, which is write-only: unlike most keys in
/// this module, there is no matching `read()`/`update()`.
///
/// # Errors
///
/// Returns an error (`ENOENT`) if `opt.prof` is `false` at runtime. The
/// `profiling` feature enables profiling support but does not set `opt.prof`;
/// configure `prof:true`, for example via `JEMALLOC_SYS_WITH_MALLOC_CONF` at
/// build time.
pub fn prof_reset(new_lg_sample: libc::size_t) -> crate::error::Result<()> {
    unsafe { crate::raw::write(b"prof.reset\0", new_lg_sample) }
}

/// Signature of a hook installable via [`set_prof_sample_hook`].
///
/// `jemalloc` invokes this hook synchronously, inline on the allocating
/// thread, immediately after it decides to sample an allocation of
/// `usable_size` bytes at `ptr` (the request was for `size` bytes;
/// `usable_size` is jemalloc's usable/rounded-up size). `backtrace` points
/// to an array of `backtrace_length` `void *` frames captured by the installed
/// [`ProfBacktraceHook`] (`backtrace_length` is `0` if
/// [`noop_prof_backtrace_hook`] is installed).
///
/// # Safety
///
/// Arguments are valid only during the call. The hook must not deallocate
/// `ptr`, retain any argument, or mutate `backtrace`. No `jemalloc` mutex is
/// held, and the hook may allocate or free other allocations; nested allocator
/// activity is excluded from profiling. Hooks may run concurrently and must
/// not unwind across the `extern "C"` boundary.
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
/// `usable_size` bytes at `ptr`.
///
/// # Safety
///
/// `ptr` is valid only during the call and must not be retained or deallocated.
/// The hook may allocate or free other allocations; nested allocator activity
/// is excluded from profiling. Hooks may run concurrently and must not unwind
/// across the `extern "C"` boundary.
pub type ProfSampleFreeHook =
    unsafe extern "C" fn(ptr: *const c_void, usable_size: libc::size_t);

/// Signature of a hook installable via [`set_prof_backtrace_hook`].
///
/// `jemalloc` invokes this hook to capture the stack trace for a sample.
///
/// # Safety
///
/// The pointers are valid only during the call and must not be retained. The
/// hook must write at most `max_length` frames into `backtrace`, store that
/// count through `backtrace_length`, support concurrent calls, and not unwind
/// across the `extern "C"` boundary.
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
/// Returns an error (`ENOENT`) if `opt.prof` is `false` at runtime; see
/// [`prof_reset`]. Note that `opt.prof` being `true` is sufficient to
/// install a hook; [`prof_active`] need not be `true` (installing while
/// inactive is a no-op until activated).
///
/// # Safety
///
/// The caller must ensure the linked `jemalloc` uses the documented
/// [`ProfSampleHook`] ABI and that `hook`, if present, upholds its contract.
/// Replaced hooks may still be in flight, so their state must remain valid.
pub unsafe fn set_prof_sample_hook(
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
///
/// # Safety
///
/// The caller must ensure the linked `jemalloc` uses the documented
/// [`ProfSampleFreeHook`] ABI and that `hook`, if present, upholds its contract.
/// Replaced hooks may still be in flight, so their state must remain valid.
pub unsafe fn set_prof_sample_free_hook(
    hook: Option<ProfSampleFreeHook>,
) -> crate::error::Result<Option<ProfSampleFreeHook>> {
    unsafe {
        crate::raw::update(b"experimental.hooks.prof_sample_free\0", hook)
    }
}

/// Installs or replaces the hook `jemalloc` calls to capture a sample's
/// backtrace, returning the previously-installed hook, if any.
///
/// Corresponds to `experimental.hooks.prof_backtrace`. Unlike
/// [`set_prof_sample_hook`]/[`set_prof_sample_free_hook`], this hook cannot
/// be uninstalled (`jemalloc` rejects a `NULL` new hook with `EINVAL`). Install
/// [`noop_prof_backtrace_hook`] instead of `jemalloc`'s default unwinder if
/// backtraces aren't wanted. If present, the returned previous hook may be
/// restored later or invoked by the replacement during a valid backtrace-hook
/// call.
///
/// # Errors
///
/// Returns an error (`ENOENT`) if `opt.prof` is `false` at runtime; see
/// [`prof_reset`].
///
/// # Safety
///
/// The caller must ensure the linked `jemalloc` uses the documented
/// [`ProfBacktraceHook`] ABI and that `hook` upholds its contract. Replaced
/// hooks may still be in flight, so their state must remain valid.
pub unsafe fn set_prof_backtrace_hook(
    hook: ProfBacktraceHook,
) -> crate::error::Result<Option<ProfBacktraceHook>> {
    unsafe {
        crate::raw::update(b"experimental.hooks.prof_backtrace\0", Some(hook))
    }
}

/// A [`ProfBacktraceHook`] that reports an empty backtrace for every
/// sample.
///
/// Installing this via [`set_prof_backtrace_hook`] disables `jemalloc`'s
/// own stack unwinding going forward: the per-allocation sampling
/// decision still happens at the configured rate (see [`lg_sample`])
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
pub unsafe extern "C" fn noop_prof_backtrace_hook(
    _backtrace: *mut *mut c_void,
    backtrace_length: *mut c_uint,
    _max_length: c_uint,
) {
    *backtrace_length = 0;
}

// Heap-allocates to force samples, so this needs a real allocator (`use_std`).
#[cfg(all(test, feature = "use_std"))]
mod hook_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static SAMPLE_HOOK_CALLS: AtomicUsize = AtomicUsize::new(0);
    static SAMPLE_FREE_HOOK_CALLS: AtomicUsize = AtomicUsize::new(0);
    static SAMPLE_BACKTRACE_LENGTH: AtomicUsize = AtomicUsize::new(usize::MAX);

    unsafe extern "C" fn counting_sample_hook(
        _ptr: *const c_void,
        _size: libc::size_t,
        _backtrace: *mut *mut c_void,
        backtrace_length: c_uint,
        _usable_size: libc::size_t,
    ) {
        SAMPLE_BACKTRACE_LENGTH
            .store(backtrace_length as usize, Ordering::SeqCst);
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
        // Requires `opt.prof`; see the `profiling` feature docs. The CI job
        // for this test bakes `prof:true` via `JEMALLOC_SYS_WITH_MALLOC_CONF`.
        if !prof::read().unwrap() {
            return;
        }
        let was_active = prof_active::update(false).unwrap();
        let prev_lg_sample = lg_sample::read().unwrap();
        SAMPLE_BACKTRACE_LENGTH.store(usize::MAX, Ordering::SeqCst);
        // lg_sample: 0 => average one sample per byte, i.e. every allocation.
        // `jemalloc` only recomputes each thread's next sample distance
        // (from the new `lg_sample`) once the current, already-primed
        // distance (drawn under whatever `lg_sample` was in effect before
        // this call, e.g. the crate's default of 512 KiB) has been
        // exhausted, so the very next allocation isn't guaranteed to sample
        // yet. Only allocations after that first one are.
        prof_reset(0).unwrap();
        let prev_backtrace =
            unsafe { set_prof_backtrace_hook(noop_prof_backtrace_hook) }
                .unwrap()
                .expect("jemalloc had no previous backtrace hook");
        let prev_sample =
            unsafe { set_prof_sample_hook(Some(counting_sample_hook)) }
                .unwrap();
        let prev_sample_free = unsafe {
            set_prof_sample_free_hook(Some(counting_sample_free_hook))
        }
        .unwrap();
        prof_active::write(true).unwrap();

        // Warm up past any sample distance primed under the previous
        // `lg_sample`. The default's geometric interval can reach ~18 MiB, so
        // burn well past that before checking for samples.
        for _ in 0..32 {
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

        prof_active::write(false).unwrap();
        unsafe { set_prof_sample_hook(prev_sample) }.unwrap();
        unsafe { set_prof_sample_free_hook(prev_sample_free) }.unwrap();
        unsafe { set_prof_backtrace_hook(prev_backtrace) }.unwrap();
        prof_reset(prev_lg_sample).unwrap();
        prof_active::write(was_active).unwrap();

        assert!(
            SAMPLE_HOOK_CALLS.load(Ordering::SeqCst) > before_sample,
            "prof_sample hook did not fire after warm-up with lg_sample=0"
        );
        assert!(
            SAMPLE_FREE_HOOK_CALLS.load(Ordering::SeqCst) > before_free,
            "prof_sample_free hook did not fire after freeing sampled allocations"
        );
        assert_eq!(SAMPLE_BACKTRACE_LENGTH.load(Ordering::SeqCst), 0);
    }
}
