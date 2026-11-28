// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use core::{
    alloc::{AllocError, Allocator, Layout},
    ptr::{self, NonNull},
    sync::atomic::{AtomicUsize, Ordering},
};

use libc::c_int;

use super::{ffi, Jemalloc};

static FIRST_EXTENT_SIZE: AtomicUsize = AtomicUsize::new(0);

#[cold]
fn load_first_extent_size() -> usize {
    let mut size = 0usize;
    let mut len = core::mem::size_of::<usize>();
    // SAFETY: this read-only mallctl writes one size_t into the output buffer.
    let status = unsafe {
        ffi::mallctl(
            b"arenas.lextent.0.size\0".as_ptr().cast(),
            ptr::addr_of_mut!(size).cast(),
            &mut len,
            ptr::null_mut(),
            0,
        )
    };
    if status != 0 || len != core::mem::size_of::<usize>() || size == 0 {
        // If the linked library can't report the boundary, let xallocx decide
        // whether an in-place resize is possible for any valid size class.
        size = 1;
    }
    // The size table is immutable. Concurrent lookups may cache the same value.
    FIRST_EXTENT_SIZE.store(size, Ordering::Relaxed);
    size
}

/// A cross-class resize can stay in place only when both size classes are
/// extent-managed. The boundary depends on the linked jemalloc configuration.
#[inline]
fn cross_class_settle_supported(old_bucket: usize, new_bucket: usize) -> bool {
    let first_extent_size = match FIRST_EXTENT_SIZE.load(Ordering::Relaxed) {
        0 => load_first_extent_size(),
        size => size,
    };
    old_bucket.min(new_bucket) >= first_extent_size
}

#[cfg(test)]
#[test]
fn resize_gate_uses_the_configured_extent_boundary() {
    use tikv_jemalloc_ctl::raw;

    let first_extent: usize = unsafe { raw::read(b"arenas.lextent.0.size\0") }.unwrap();
    let nbins: libc::c_uint = unsafe { raw::read(b"arenas.nbins\0") }.unwrap();
    let mut bin_size_mib = [0; 4];
    raw::name_to_mib(b"arenas.bin.0.size\0", &mut bin_size_mib).unwrap();
    bin_size_mib[2] = nbins as usize - 1;
    let last_slab: usize = unsafe { raw::read_mib(&bin_size_mib) }.unwrap();

    assert!(!cross_class_settle_supported(last_slab, first_extent));
    assert!(!cross_class_settle_supported(first_extent, last_slab));
    assert!(cross_class_settle_supported(first_extent, first_extent * 2));
}

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
        if new_layout.size() == 0 {
            // Shrinking down to nothing: serve a fresh zero-sized block for
            // the requested alignment (which no real allocation could meet)
            // and release the original one under its own layout. Doing this
            // before any address/alignment check is essential -- a relocation
            // would end up allocating zero bytes "in place", which the raw
            // allocation primitives refuse by contract.
            let block = Self::zero_block(&new_layout);
            // SAFETY: invalidates and releases the original block.
            unsafe { self.deallocate(ptr, old_layout) };
            return Ok(block);
        }
        // Since growing/shrinking may keep the base address unchanged, it
        // cannot satisfy alignment requirements beyond those already met by
        // that address.
        let address = ptr.as_ptr() as usize;
        if !address.is_multiple_of(new_layout.align()) {
            return self.relocate_and_copy(ptr, old_layout, new_layout, grow_zeroed);
        }
        // Keep the backing size class consistent with the layout. A class
        // change can stay in place only if jemalloc resizes the backing block.
        let old_flags = ffi::MALLOCX_ALIGN(old_layout.align());
        let new_flags = ffi::MALLOCX_ALIGN(new_layout.align());
        let old_bucket = unsafe { ffi::nallocx(old_layout.size(), old_flags) };
        let new_bucket = unsafe { ffi::nallocx(new_layout.size(), new_flags) };
        let resized = old_bucket == new_bucket
            || (old_layout.size() != new_layout.size()
                && cross_class_settle_supported(old_bucket, new_bucket)
                // SAFETY: ptr is live and the gate excludes zero size classes.
                // Request the target class so alignment rounding also agrees
                // with nallocx.
                && unsafe {
                    ffi::xallocx(ptr.as_ptr().cast(), new_bucket, 0, new_flags) == new_bucket
                });

        if !resized {
            return self.relocate_and_copy(ptr, old_layout, new_layout, grow_zeroed);
        }
        if grow_zeroed && new_layout.size() > old_layout.size() {
            Self::zero_extension(ptr, old_layout.size(), new_layout.size());
        }
        Ok(Self::own_block(ptr, new_layout.size()))
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
        let flags = if grow_zeroed { ffi::MALLOCX_ZERO } else { 0 };
        let target = Self::allocate_raw(new_layout, flags).ok_or(AllocError)?;
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

    // mallocx makes the rounded size class usable, including when a subsequent
    // grow stays in that class. The slice returned to Rust still reports the
    // requested length.
    #[inline]
    fn allocate_raw(layout: Layout, flags: c_int) -> Option<NonNull<u8>> {
        assume!(layout.size() != 0);
        let flags = ffi::MALLOCX_ALIGN(layout.align()) | flags;
        NonNull::new(unsafe { ffi::mallocx(layout.size(), flags) }.cast())
    }
}

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
        Self::allocate_raw(layout, 0)
            .map(|base| Jemalloc::own_block(base, layout.size()))
            .ok_or(AllocError)
    }

    #[inline]
    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        if layout.size() == 0 {
            return Ok(Jemalloc::zero_block(&layout));
        }
        // SAFETY: nonzero layouts only, per the guard above.
        Self::allocate_raw(layout, ffi::MALLOCX_ZERO)
            .map(|base| Jemalloc::own_block(base, layout.size()))
            .ok_or(AllocError)
    }

    #[inline]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        if layout.size() != 0 {
            // SAFETY: the caller supplies a live block. After an in-place
            // resize, the fitting layout can differ from jemalloc's original
            // request in both size and alignment.
            unsafe { ffi::dallocx(ptr.as_ptr().cast(), 0) };
        }
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
