//! Bulk statistics output.

use libc::{c_char, c_void};
use std::any::Any;
use std::ffi::CStr;
use std::io::{self, Write};
use std::panic::{self, AssertUnwindSafe};

/// Statistics configuration.
///
/// All options default to `false`.
#[derive(Copy, Clone, Default)]
#[non_exhaustive]
pub struct Options {
    /// If set, the output will be JSON-formatted.
    ///
    /// This corresponds to the `J` character.
    pub json_format: bool,

    /// Skips the general configuration section in
    /// [`stats_print_with_environment`]. [`stats_print`] always skips it.
    ///
    /// This corresponds to the `g` character.
    pub skip_constants: bool,

    /// If set, merged information about arenas will be skipped.
    ///
    /// This corresponds to the `m` character.
    pub skip_merged_arenas: bool,

    /// If set, information about individual arenas will be skipped.
    ///
    /// This corresponds to the `a` character.
    pub skip_per_arena: bool,

    /// If set, information about individual size classes for bins will be skipped.
    ///
    /// This corresponds to the `b` character.
    pub skip_bin_size_classes: bool,

    /// If set, information about individual size classes for large objects will be skipped.
    ///
    /// This corresponds to the `l` character.
    pub skip_large_size_classes: bool,

    /// If set, mutex statistics will be skipped.
    ///
    /// This corresponds to the `x` character.
    pub skip_mutex_statistics: bool,
}

struct State<W> {
    writer: W,
    error: io::Result<()>,
    panic: Result<(), Box<dyn Any + Send>>,
}

extern "C" fn callback<W>(opaque: *mut c_void, buf: *const c_char)
where
    W: Write,
{
    unsafe {
        let state = &mut *(opaque as *mut State<W>);
        if state.error.is_err() || state.panic.is_err() {
            return;
        }

        let buf = CStr::from_ptr(buf);
        match panic::catch_unwind(AssertUnwindSafe(|| {
            state.writer.write_all(buf.to_bytes())
        })) {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => state.error = Err(e),
            Err(e) => state.panic = Err(e),
        }
    }
}

/// Writes allocator statistics with the general configuration section omitted.
///
/// The omitted section includes jemalloc's cached environment configuration.
/// With the vendored jemalloc, enable the `stats` feature to include runtime
/// metrics. With `use_std` alone, the text report contains only its opening
/// and closing markers and JSON contains an empty `jemalloc` object.
pub fn stats_print<W>(writer: W, mut options: Options) -> io::Result<()>
where
    W: Write,
{
    options.skip_constants = true;
    // SAFETY: The `g` option skips the section that reads cached strings.
    unsafe { stats_print_with_environment(writer, options) }
}

/// Writes allocator statistics with the general configuration section enabled
/// by default. The section includes the allocator's startup environment
/// configuration, such as `MALLOC_CONF` or its prefixed equivalent.
///
/// # Safety
///
/// With [`Options::skip_constants`] set to `false`, the caller must ensure
/// that the configuration strings cached by jemalloc remain valid and
/// immutable throughout the call, including while `writer` runs. If jemalloc
/// cached a string from the process environment, its storage must remain
/// valid from jemalloc initialization through the end of this call.
///
/// ```compile_fail
/// use tikv_jemalloc_ctl::stats_print::{stats_print_with_environment, Options};
///
/// stats_print_with_environment(Vec::new(), Options::default()).unwrap();
/// ```
#[allow(clippy::cast_possible_wrap)]
pub unsafe fn stats_print_with_environment<W>(
    writer: W,
    options: Options,
) -> io::Result<()>
where
    W: Write,
{
    unsafe {
        let mut state = State {
            writer,
            error: Ok(()),
            panic: Ok(()),
        };
        let mut opts = [0; 8];
        let mut i = 0;
        if options.json_format {
            opts[i] = b'J' as c_char;
            i += 1;
        }
        if options.skip_constants {
            opts[i] = b'g' as c_char;
            i += 1;
        }
        if options.skip_merged_arenas {
            opts[i] = b'm' as c_char;
            i += 1;
        }
        if options.skip_per_arena {
            opts[i] = b'a' as c_char;
            i += 1;
        }
        if options.skip_bin_size_classes {
            opts[i] = b'b' as c_char;
            i += 1;
        }
        if options.skip_large_size_classes {
            opts[i] = b'l' as c_char;
            i += 1;
        }
        if options.skip_mutex_statistics {
            opts[i] = b'x' as c_char;
            i += 1;
        }
        opts[i] = 0;

        tikv_jemalloc_sys::malloc_stats_print(
            Some(callback::<W>),
            &mut state as *mut _ as *mut c_void,
            opts.as_ptr(),
        );
        if let Err(e) = state.panic {
            panic::resume_unwind(e);
        }
        state.error
    }
}
