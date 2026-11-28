// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Bindings for jemalloc as an allocator
//!
//! This crate provides bindings to jemalloc as a memory allocator for Rust.
//! This crate mainly exports, one type, `Jemalloc`, which implements the
//! `GlobalAlloc` trait, and optionally the `core::alloc::Allocator` trait,
//! and is suitable both as a memory allocator and as a global allocator.
//!
//! `GlobalAlloc` uses the original layout to pass a sized deallocation hint
//! to jemalloc. `Allocator` releases blocks without a hint because a valid
//! layout can change through an in-place resize. Callers using the raw
//! `tikv-jemalloc-sys` bindings must uphold the contract of any sized
//! deallocation they perform.

// TODO: rename the following lint on next minor bump
#![allow(renamed_and_removed_lints)]
#![deny(missing_docs, broken_intra_doc_links)]
#![no_std]

use core::alloc::{GlobalAlloc, Layout};

use libc::{c_int, c_void};

/// jemalloc's tiny size classes are powers of two. Once the requested size
/// reaches the alignment, plain malloc can satisfy it up to the configured
/// alignment bound.
fn layout_to_flags(align: usize, size: usize) -> c_int {
    if align <= ffi::MALLOC_ALIGNMENT && align <= size {
        0
    } else {
        ffi::MALLOCX_ALIGN(align)
    }
}

// Assumes a condition that always must hold.
macro_rules! assume {
    ($e:expr) => {
        debug_assert!($e);
        if !($e) {
            // SAFETY: the assumed condition is documented as always holding;
            // recent rustc versions made `unreachable_unchecked` an unsafe fn.
            unsafe {
                core::hint::unreachable_unchecked();
            }
        }
    };
}

/// Handle to the jemalloc allocator
///
/// This type implements the `GlobalAlloc` trait, allowing usage as a global
/// allocator.
///
/// With the `alloc_trait` feature on Rust 1.100 or later, it also implements
/// `core::alloc::Allocator` for direct use in collections such as `Vec`.
#[derive(Copy, Clone, Default, Debug)]
pub struct Jemalloc;

unsafe impl GlobalAlloc for Jemalloc {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        assume!(layout.size() != 0);
        let flags = layout_to_flags(layout.align(), layout.size());
        let ptr = if flags == 0 {
            ffi::malloc(layout.size())
        } else {
            ffi::mallocx(layout.size(), flags)
        };
        ptr as *mut u8
    }

    #[inline]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        assume!(layout.size() != 0);
        let flags = layout_to_flags(layout.align(), layout.size());
        let ptr = if flags == 0 {
            ffi::calloc(1, layout.size())
        } else {
            ffi::mallocx(layout.size(), flags | ffi::MALLOCX_ZERO)
        };
        ptr as *mut u8
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        assume!(!ptr.is_null());
        assume!(layout.size() != 0);
        let flags = layout_to_flags(layout.align(), layout.size());
        ffi::sdallocx(ptr as *mut c_void, layout.size(), flags)
    }

    #[inline]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        assume!(layout.size() != 0);
        assume!(new_size != 0);
        let flags = layout_to_flags(layout.align(), new_size);
        let ptr = if flags == 0 {
            ffi::realloc(ptr as *mut c_void, new_size)
        } else {
            ffi::rallocx(ptr as *mut c_void, new_size, flags)
        };
        ptr as *mut u8
    }
}

#[cfg(feature = "alloc_trait")]
mod allocator;

/// Return the usable size of the allocation pointed to by ptr.
///
/// The return value may be larger than the size that was requested during allocation.
/// This function is not a mechanism for in-place `realloc()`;
/// rather it is provided solely as a tool for introspection purposes.
/// Any discrepancy between the requested allocation size
/// and the size reported by this function should not be depended on,
/// since such behavior is entirely implementation-dependent.
///
/// # Safety
///
/// `ptr` must have been allocated by `Jemalloc` and must not have been freed yet.
pub unsafe fn usable_size<T>(ptr: *const T) -> usize {
    ffi::malloc_usable_size(ptr as *const c_void)
}

/// Raw bindings to jemalloc
mod ffi {
    pub use tikv_jemalloc_sys::*;
}
