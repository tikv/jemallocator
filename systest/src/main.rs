// TODO: remove systest
#![allow(bad_style, improper_ctypes, dead_code, unused_imports, deref_nullptr)]
#![allow(clippy::all)]

use std::alloc::System;

#[global_allocator]
static A: System = System;

use jemalloc_sys::*;
use libc::{c_char, c_int, c_void};

include!(concat!(env!("OUT_DIR"), "/all.rs"));
