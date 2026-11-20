//! Compile-time checks for the public API, included only during doctests.
//!
//! A key can only be read as its declared type.
//!
//! ```compile_fail
//! use tikv_jemalloc_ctl::{epoch, Access};
//!
//! let _: bool = epoch::name().read().unwrap();
//! ```
//!
//! Custom key construction requires `unsafe`.
//!
//! ```compile_fail
//! use tikv_jemalloc_ctl::Name;
//!
//! let _ = Name::<u64>::new(b"epoch\0");
//! ```
//!
//! A MIB's indices cannot be changed through safe access.
//!
//! ```compile_fail
//! use tikv_jemalloc_ctl::epoch;
//!
//! let mut mib = epoch::name().mib().unwrap();
//! mib[0] = 0;
//! ```
//!
//! A generated MIB wrapper's indices are private.
//!
//! ```compile_fail
//! use tikv_jemalloc_ctl::version_mib;
//!
//! let _ = version_mib([0]);
//! ```
//!
//! Reading a cached thread counter pointer requires `unsafe`.
//!
//! ```compile_fail
//! use tikv_jemalloc_ctl::thread::allocatedp;
//!
//! let counter = allocatedp::read().unwrap();
//! let _ = counter.get();
//! ```

#[cfg(feature = "use_std")]
/// Printing allocator statistics with environment data requires `unsafe`.
///
/// ```compile_fail
/// use tikv_jemalloc_ctl::stats_print::{stats_print_with_environment, Options};
///
/// stats_print_with_environment(Vec::new(), Options::default()).unwrap();
/// ```
mod stats_print {}
