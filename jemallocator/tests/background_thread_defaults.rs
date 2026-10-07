//! Test background threads run-time default settings.

use tikv_jemallocator::Jemalloc;

#[global_allocator]
static A: Jemalloc = Jemalloc;

// Returns true if background threads are enabled.
fn background_threads() -> bool {
    tikv_jemalloc_ctl::opt::background_thread::read().unwrap()
}

#[test]
fn background_threads_runtime_defaults() {
    if !cfg!(feature = "background_threads_runtime_support") {
        // If the crate was compiled without background thread support,
        // then background threads are always disabled at run-time by default:
        assert!(!background_threads());
        return;
    }

    // `background_threads` is ignored on targets in `NO_BG_THREAD_TARGETS`.
    let supported = !cfg!(target_env = "musl") && !cfg!(target_vendor = "apple");
    assert_eq!(
        background_threads(),
        cfg!(feature = "background_threads") && supported
    );
}
