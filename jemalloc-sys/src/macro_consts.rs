// Values of jemalloc's C preprocessor macros.
//
// This file is `include!`d by both `lib.rs` and `build.rs`; the build script
// checks every value here against the vendored jemalloc headers, so a jemalloc
// update that changes one of them fails the build instead of silently
// diverging.

/// Arena index that selects *all* arenas in the `arena.<i>.{purge,decay,dss}`
/// and `stats.arenas.<i>.*` `mallctl` interfaces.
///
/// For example, `arena.4096.purge` purges unused dirty pages of every arena:
///
/// ```no_run
/// use std::ptr;
/// use tikv_jemalloc_sys::{mallctl, MALLCTL_ARENAS_ALL};
///
/// let name = format!("arena.{}.purge\0", MALLCTL_ARENAS_ALL);
/// let ret = unsafe {
///     mallctl(
///         name.as_ptr() as *const _,
///         ptr::null_mut(),
///         ptr::null_mut(),
///         ptr::null_mut(),
///         0,
///     )
/// };
/// assert_eq!(ret, 0);
/// ```
pub const MALLCTL_ARENAS_ALL: usize = 4096;
