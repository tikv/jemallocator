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

// TODO: rename the following lint on next minor bump
#![allow(renamed_and_removed_lints)]
#![deny(missing_docs, broken_intra_doc_links)]
#![no_std]

#[cfg(feature = "alloc_trait")]
use core::alloc::{AllocError, Allocator};
use core::alloc::{GlobalAlloc, Layout};
#[cfg(feature = "alloc_trait")]
use core::ptr::NonNull;

use libc::{c_int, c_void};

// This constant equals _Alignof(max_align_t) and is platform-specific. It
// contains the _maximum_ alignment that the memory allocation returned by the
// C standard library memory allocation APIs (e.g. `malloc`) are guaranteed to
// have.
//
// In C, there are no ZSTs, and the size of all types is a multiple of their
// alignment (size >= align). So for allocations with size <=
// _Alignof(max_align_t), the malloc-APIs return memory whose alignment is
// either the requested size if its a power-of-two, or the next smaller
// power-of-two.
#[cfg(any(target_arch = "arm", target_arch = "mips", target_arch = "powerpc"))]
const ALIGNOF_MAX_ALIGN_T: usize = 8;
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "powerpc64",
    target_arch = "loongarch64",
    target_arch = "mips64",
    target_arch = "riscv32",
    target_arch = "riscv64",
    target_arch = "s390x",
    target_arch = "sparc64"
))]
const ALIGNOF_MAX_ALIGN_T: usize = 16;

/// If `align` is less than `_Alignof(max_align_t)`, and if the requested
/// allocation `size` is larger than the alignment, we are guaranteed to get a
/// suitably aligned allocation by default, without passing extra flags, and
/// this function returns `0`.
///
/// Otherwise, it returns the alignment flag to pass to the jemalloc APIs.
fn layout_to_flags(align: usize, size: usize) -> c_int {
    if align <= ALIGNOF_MAX_ALIGN_T && align <= size {
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
/// When the `alloc_trait` feature is enabled, it also implements the
/// recently stabilized `core::alloc::Allocator` trait, allowing usage
/// directly in collections such as `Vec`. Enabling that feature requires a
/// toolchain that already carries the freshly landed allocator API.
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
impl Jemalloc {
    /// Construct a slice pointer covering exactly `len` bytes starting at `base`.
    ///
    /// # Safety
    ///
    /// `base` must be properly aligned, non-null, and valid for reading and
    /// writing `len` bytes; for zero-length blocks no validity requirement on
    /// the pointed-to memory applies beyond alignment.
    #[inline]
    const fn own_block(base: NonNull<u8>, len: usize) -> NonNull<[u8]> {
        // Building the fat pointer straight from the thin base preserves the
        // caller-upheld validity plus full, write-capable provenance, unlike
        // routing through a shared `&[u8]`.
        NonNull::slice_from_raw_parts(base, len)
    }

    /// A zero-sized block for `layout`: an aligned non-null pointer that does
    /// not reference jemalloc-managed memory.
    #[inline]
    const fn zero_block(layout: &Layout) -> NonNull<[u8]> {
        // SAFETY: `Layout::dangling_ptr` returns a non-null pointer that
        // satisfies the alignment of `layout`.
        Self::own_block(layout.dangling_ptr(), 0)
    }

    /// Grow/shrink a live block in place when possible, moving it otherwise.
    ///
    /// Returns an error leaving the original block valid and unmodified on
    /// failure.
    #[inline]
    unsafe fn resize_blocks(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
        grow_zeroed: bool,
    ) -> Result<NonNull<[u8]>, AllocError> {
        if old_layout.size() == 0 {
            // Block came from a zero-sized allocation: nothing lives there,
            // so just produce a fresh block for the new layout.
            if grow_zeroed {
                return self.allocate_zeroed(new_layout);
            }
            return self.allocate(new_layout);
        }
        // Since growing/shrinking may keep the base address unchanged, it
        // cannot satisfy alignment requirements beyond those already met by
        // that address.
        let address = ptr.as_ptr() as usize;
        if !address.is_multiple_of(new_layout.align()) {
            return self.relocate_and_copy(ptr, old_layout, new_layout, grow_zeroed);
        }
        // jemalloc accounts for every allocation under its *quantized*
        // usable size (size bucket). Deallocating a pointer whose eventual
        // layout maps onto a different bucket than the recorded one feeds
        // the wrong size to jemalloc's accounting and corrupts its internal
        // bookkeeping (sized frees get routed to the wrong caches). So a
        // pointer that stays put may only keep serving layouts of the same
        // bucket unless the resize updates the record itself. `nallocx`
        // reports authoritative buckets without touching allocations.
        // SAFETY: both sizes are nonzero here and alignments are powers of
        // two per the `Layout` constructors; `nallocx` inspects its inputs
        // only.
        let old_flags = layout_to_flags(old_layout.align(), old_layout.size());
        let new_flags = layout_to_flags(new_layout.align(), new_layout.size());
        let old_bucket = unsafe { ffi::nallocx(old_layout.size(), old_flags) };
        let new_bucket = unsafe { ffi::nallocx(new_layout.size(), new_flags) };
        match new_layout.size().cmp(&old_layout.size()) {
            core::cmp::Ordering::Greater => {
                // Growing. When the request fits the recorded bucket nothing
                // changes on jemalloc's side, and later sized frees stay
                // consistent with the untouched record.
                let extended = if new_bucket == old_bucket {
                    true
                } else {
                    // Ask jemalloc to extend in place where it legally can
                    // (large extents do), which refreshes the record; the
                    // call reports exactly the new bucket's size on success.
                    // SAFETY: the block is currently allocated by this
                    // allocator with a fitting layout.
                    unsafe {
                        ffi::xallocx(ptr.as_ptr() as *mut c_void, new_layout.size(), 0, new_flags)
                            == new_bucket
                    }
                };
                if extended {
                    if grow_zeroed {
                        Self::zero_extension(ptr, old_layout.size(), new_layout.size());
                    }
                    return Ok(Self::own_block(ptr, new_layout.size()));
                }
                self.relocate_and_copy(ptr, old_layout, new_layout, grow_zeroed)
            }
            core::cmp::Ordering::Less => {
                // Shrinking cannot fail: every byte reached now was backed
                // before.
                if new_layout.size() == 0 {
                    let block = Self::zero_block(&new_layout);
                    // SAFETY: invalidates and releases the original block.
                    unsafe { self.deallocate(ptr, old_layout) };
                    return Ok(block);
                }
                if new_bucket == old_bucket {
                    // Same bucket: keep the record and reuse the pointer.
                    return Ok(Self::own_block(ptr, new_layout.size()));
                }
                // Ask jemalloc best-effort to move the block down a bucket
                // in place so surplus pages can decay/purge again; success
                // lands exactly on the new bucket. Slab-managed small
                // allocations can never do that, so fall back to moving the
                // data manually -- keeping the pointer would desync jemalloc
                // from the smaller layout it is about to serve up.
                // SAFETY: the block is currently allocated by this allocator
                // with a fitting layout.
                let moved_down = unsafe {
                    ffi::xallocx(ptr.as_ptr() as *mut c_void, new_layout.size(), 0, new_flags)
                        == new_bucket
                };
                if moved_down {
                    return Ok(Self::own_block(ptr, new_layout.size()));
                }
                self.relocate_and_copy(ptr, old_layout, new_layout, grow_zeroed)
            }
            core::cmp::Ordering::Equal => {
                // Sizes agree, so only alignment constraints may differ and
                // the address was verified above. Staying put is consistent
                // with the record only for layouts sharing its bucket.
                if new_bucket == old_bucket {
                    return Ok(Self::own_block(ptr, new_layout.size()));
                }
                self.relocate_and_copy(ptr, old_layout, new_layout, grow_zeroed)
            }
        }
    }

    /// Zero the extended span of an in-place grown block, upholding the
    /// `grow_zeroed` contract for its uninitialized tail.
    #[inline]
    unsafe fn zero_extension(ptr: NonNull<u8>, old_size: usize, new_size: usize) {
        // SAFETY: the tail lies within the backing range covered either by
        // the matched bucket of the existing record or by the successful
        // in-place growth that preceded this call.
        unsafe {
            (ptr.add(old_size).as_ptr()).write_bytes(0, new_size - old_size);
        }
    }

    /// Move a block to a fresh allocation for `new_layout`, preserving the
    /// contents common to both layouts. Leaves the original block allocated
    /// and untouched on failure.
    #[inline]
    unsafe fn relocate_and_copy(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
        grow_zeroed: bool,
    ) -> Result<NonNull<[u8]>, AllocError> {
        // The destination comes straight from the allocation primitives so
        // that the underlying thin pointer is directly available for the
        // copy.
        // SAFETY: nonzero layouts only, per the trait contracts of
        // `grow`/`shrink` callers; a zero-sized destination would have been
        // handled by the early branches of `resize_blocks` instead.
        let target = if grow_zeroed {
            self.allocate_zeroed_raw(new_layout)
        } else {
            self.allocate_raw(new_layout)
        }
        .ok_or(AllocError)?;
        // SAFETY: both blocks are currently allocated by equivalent
        // allocators and disjoint from each other; the copied span is the
        // intersection of their requested (and hence backed) ranges.
        unsafe {
            core::ptr::copy_nonoverlapping(
                ptr.as_ptr(),
                target.as_ptr(),
                core::cmp::min(old_layout.size(), new_layout.size()),
            );
        }
        // SAFETY: invalidates and releases the original block now that its
        // preserved bytes have been moved.
        unsafe { self.deallocate(ptr, old_layout) };
        Ok(Self::own_block(target, new_layout.size()))
    }

    /// Thin-pointer flavour of `GlobalAlloc::alloc`; see it for safety notes.
    #[inline]
    fn allocate_raw(&self, layout: Layout) -> Option<NonNull<u8>> {
        assume!(layout.size() != 0);
        let flags = layout_to_flags(layout.align(), layout.size());
        let ptr = if flags == 0 {
            unsafe { ffi::malloc(layout.size()) }
        } else {
            unsafe { ffi::mallocx(layout.size(), flags) }
        };
        NonNull::new(ptr as *mut u8)
    }

    /// Thin-pointer flavour of `GlobalAlloc::alloc_zeroed`; see it for safety notes.
    #[inline]
    fn allocate_zeroed_raw(&self, layout: Layout) -> Option<NonNull<u8>> {
        assume!(layout.size() != 0);
        let flags = layout_to_flags(layout.align(), layout.size());
        let ptr = if flags == 0 {
            unsafe { ffi::calloc(1, layout.size()) }
        } else {
            unsafe { ffi::mallocx(layout.size(), flags | ffi::MALLOCX_ZERO) }
        };
        NonNull::new(ptr as *mut u8)
    }
}

#[cfg(feature = "alloc_trait")]
unsafe impl Allocator for Jemalloc {
    /// # Safety
    ///
    /// All operations forward to jemalloc's allocation functions with matching
    /// sizes and alignments. Every block handed out here reports exactly the
    /// requested length, stays valid until released by one of these same
    /// methods, and blocks are never shared between distinct allocations, so
    /// the trait invariants about currently-allocated/invalidated blocks and
    /// disjointness hold. Instances of `Jemalloc` are pairwise equivalent
    /// because the type carries no state.
    #[inline]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        if layout.size() == 0 {
            return Ok(Jemalloc::zero_block(&layout));
        }
        // SAFETY: nonzero layouts only, per the guard above.
        self.allocate_raw(layout)
            .map(|base| Jemalloc::own_block(base, layout.size()))
            .ok_or(AllocError)
    }

    #[inline]
    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        if layout.size() == 0 {
            return Ok(Jemalloc::zero_block(&layout));
        }
        // SAFETY: nonzero layouts only, per the guard above.
        self.allocate_zeroed_raw(layout)
            .map(|base| Jemalloc::own_block(base, layout.size()))
            .ok_or(AllocError)
    }

    #[inline]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        if layout.size() == 0 {
            // Zero-sized blocks were never backed by jemalloc memory.
            return;
        }
        // SAFETY: mirrors `GlobalAlloc::dealloc`, upholding its contract by
        // virtue of this method's contract.
        unsafe { GlobalAlloc::dealloc(self, ptr.as_ptr(), layout) }
    }

    #[inline]
    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        debug_assert!(new_layout.size() >= old_layout.size());
        self.resize_blocks(ptr, old_layout, new_layout, false)
    }

    #[inline]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        debug_assert!(new_layout.size() >= old_layout.size());
        self.resize_blocks(ptr, old_layout, new_layout, true)
    }

    #[inline]
    unsafe fn shrink(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        debug_assert!(new_layout.size() <= old_layout.size());
        self.resize_blocks(ptr, old_layout, new_layout, false)
    }
}

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
