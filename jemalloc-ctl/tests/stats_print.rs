#![cfg(feature = "use_std")]

use tikv_jemalloc_ctl::stats_print::{
    stats_print, stats_print_with_environment, Options,
};

#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[test]
fn safe_print_omits_environment_with_default_options() {
    let mut buf = vec![];
    stats_print(&mut buf, Options::default()).unwrap();
    let output = String::from_utf8(buf).unwrap();
    assert!(output.contains("Begin jemalloc statistics"));
    assert!(!output.contains("Environment variable MALLOC_CONF"));
}

#[test]
fn full_print_includes_environment() {
    let mut buf = vec![];
    // SAFETY: The test process leaves its environment and jemalloc's
    // startup configuration unchanged.
    unsafe { stats_print_with_environment(&mut buf, Options::default()) }
        .unwrap();
    let output = String::from_utf8(buf).unwrap();
    assert!(output.contains("Environment variable MALLOC_CONF"));
}

#[test]
fn all_options() {
    let mut buf = vec![];
    let mut options = Options::default();
    options.json_format = true;
    options.skip_constants = true;
    options.skip_merged_arenas = true;
    options.skip_per_arena = true;
    options.skip_bin_size_classes = true;
    options.skip_large_size_classes = true;
    options.skip_mutex_statistics = true;
    stats_print(&mut buf, options).unwrap();
    let output = String::from_utf8(buf).unwrap();
    assert!(!output.contains(r#""env_var":"#));
    #[cfg(feature = "stats")]
    assert!(output.contains(r#""allocated":"#));
}
