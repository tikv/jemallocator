use libc::{c_uint, c_void};
use std::sync::atomic::{AtomicUsize, Ordering};
use tikv_jemalloc_ctl::{
    background_thread,
    profiling::{
        lg_sample, noop_prof_backtrace_hook, prof, prof_active, prof_reset,
        set_prof_backtrace_hook, set_prof_sample_free_hook,
        set_prof_sample_hook, PROF_RESET_MAX_LG_SAMPLE,
    },
};

#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

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
    SAMPLE_BACKTRACE_LENGTH.store(backtrace_length as usize, Ordering::SeqCst);
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
fn main() {
    // Requires `opt.prof`; see the `profiling` feature docs. The CI job
    // for this test bakes `prof:true` via `JEMALLOC_SYS_WITH_MALLOC_CONF`.
    if !prof::read().unwrap() {
        return;
    }
    // Cargo runs this binary without the parallel Rust test harness.
    if background_thread::read().unwrap_or(false) {
        background_thread::write(false).unwrap();
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
    // SAFETY: This process only uses jemalloc on the main thread during
    // the reset, with any jemalloc background threads stopped.
    unsafe { prof_reset(0) }.unwrap();
    let prev_backtrace =
        unsafe { set_prof_backtrace_hook(noop_prof_backtrace_hook) }
            .unwrap()
            .expect("jemalloc had no previous backtrace hook");
    let prev_sample =
        unsafe { set_prof_sample_hook(Some(counting_sample_hook)) }.unwrap();
    let prev_sample_free =
        unsafe { set_prof_sample_free_hook(Some(counting_sample_free_hook)) }
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
    // SAFETY: The process still has exclusive access to jemalloc for all resets.
    unsafe { prof_reset(PROF_RESET_MAX_LG_SAMPLE) }.unwrap();
    assert_eq!(lg_sample::read().unwrap(), PROF_RESET_MAX_LG_SAMPLE);
    assert!(unsafe { prof_reset(PROF_RESET_MAX_LG_SAMPLE + 1) }.is_err());
    assert_eq!(lg_sample::read().unwrap(), PROF_RESET_MAX_LG_SAMPLE);
    unsafe { prof_reset(prev_lg_sample) }.unwrap();
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
