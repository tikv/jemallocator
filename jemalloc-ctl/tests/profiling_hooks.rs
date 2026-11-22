use libc::{c_uint, c_void};
use std::alloc::{GlobalAlloc, Layout};
use std::sync::atomic::{AtomicUsize, Ordering};
use tikv_jemalloc_ctl::{
    background_thread,
    profiling::{
        lg_sample, noop_prof_backtrace_hook, prof, prof_active, prof_reset,
        set_prof_backtrace_hook, set_prof_sample_free_hook,
        set_prof_sample_hook, PROF_RESET_MAX_LG_SAMPLE,
    },
    Access, Name,
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

fn alloc_and_free(size: usize) {
    assert!(size > 0);
    let layout = Layout::array::<u8>(size).unwrap();
    // Call jemalloc directly so the test can observe both hooks.
    // SAFETY: The layout is nonzero, and each allocation is freed with the
    // same allocator and layout after checking for null.
    unsafe {
        let ptr = ALLOC.alloc(layout);
        assert!(!ptr.is_null(), "profiling test allocation failed");
        ALLOC.dealloc(ptr, layout);
    }
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
    // A fresh thread draws its first sample interval with lg_sample=0.
    // The main thread may still be waiting on an interval from before the reset.
    let (sampled, freed) = std::thread::spawn(|| {
        // SAFETY: This key reads and writes a bool on the calling thread.
        let thread_prof_active =
            unsafe { Name::<bool>::new(b"thread.prof.active\0") }.unwrap();
        thread_prof_active.write(true).unwrap();
        prof_active::write(true).unwrap();

        let before_sample = SAMPLE_HOOK_CALLS.load(Ordering::SeqCst);
        let before_free = SAMPLE_FREE_HOOK_CALLS.load(Ordering::SeqCst);
        alloc_and_free(4096);
        let sampled = SAMPLE_HOOK_CALLS.load(Ordering::SeqCst) > before_sample;
        let freed =
            SAMPLE_FREE_HOOK_CALLS.load(Ordering::SeqCst) > before_free;
        prof_active::write(false).unwrap();
        (sampled, freed)
    })
    .join()
    .unwrap();

    unsafe { set_prof_sample_hook(prev_sample) }.unwrap();
    unsafe { set_prof_sample_free_hook(prev_sample_free) }.unwrap();
    unsafe { set_prof_backtrace_hook(prev_backtrace) }.unwrap();
    assert_eq!(lg_sample::read().unwrap(), 0);
    // SAFETY: The worker has joined and jemalloc background threads are stopped.
    unsafe { prof_reset(PROF_RESET_MAX_LG_SAMPLE + 1) }.unwrap();
    assert_eq!(lg_sample::read().unwrap(), PROF_RESET_MAX_LG_SAMPLE);
    unsafe { prof_reset(0) }.unwrap();
    unsafe { prof_reset(PROF_RESET_MAX_LG_SAMPLE) }.unwrap();
    assert_eq!(lg_sample::read().unwrap(), PROF_RESET_MAX_LG_SAMPLE);
    unsafe { prof_reset(prev_lg_sample) }.unwrap();
    prof_active::write(was_active).unwrap();

    assert!(sampled, "prof_sample hook did not fire with lg_sample=0");
    assert!(
        freed,
        "prof_sample_free hook did not fire after freeing sampled allocations"
    );
    assert_eq!(SAMPLE_BACKTRACE_LENGTH.load(Ordering::SeqCst), 0);
}
