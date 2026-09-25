//! Tests for the stabilized `core::alloc::Allocator` implementation of
//! `Jemalloc`. The target is empty unless the `alloc_trait` feature is
//! enabled, in which case it requires a toolchain carrying that stable API.
//! Block base pointers are recovered in `base()` from stable pointer helpers;
//! the still-unstable `as_mut_ptr()` / `as_non_null_ptr()` accessors
//! (rust-lang/rust#74265) are deliberately avoided so these tests compile
//! wherever the trait itself does.

#![cfg(feature = "alloc_trait")]

use core::alloc::{Allocator, Layout};
use core::ptr::NonNull;
use tikv_jemallocator::Jemalloc;

#[global_allocator]
static A: Jemalloc = Jemalloc;

fn layout(size: usize, align: usize) -> Layout {
    Layout::from_size_align(size, align).expect("valid layout")
}

/// Base data pointer of a block.
fn base(block: &NonNull<[u8]>) -> *const u8 {
    // SAFETY: `as_ptr` hands over the block's own valid data pointer (a fat
    // `*mut [T]`); thinning it only reads the address part of that value. The
    // const-cast drops unique tagging on memory this process owns exclusively.
    unsafe { (*block.as_ptr()).as_ptr() }
}

/// Rebuild a mutable slice over a block's covered range.
#[allow(clippy::mut_from_ref)] // the block's memory belongs exclusively to us;
                               // the returned view aliases nothing
fn slice_of(block: &NonNull<[u8]>) -> &mut [u8] {
    // SAFETY: blocks cover exactly their reported length of initialized,
    // writable memory owned by this process; no other handles to that memory
    // exist, so re-tagging the shared base pointer as unique is sound.
    unsafe { core::slice::from_raw_parts_mut(base(block) as *mut u8, block.len()) }
}

/// Reconstruct the `NonNull<T>` argument expected by the `Allocator` methods.
fn base_nn(block: &NonNull<[u8]>) -> NonNull<u8> {
    // SAFETY: allocated blocks always start at a non-null, aligned address.
    unsafe { NonNull::new_unchecked(base(block) as *mut u8) }
}

#[test]
fn roundtrip_many_layouts() {
    unsafe {
        for &size in &[1, 3, 7, 8, 16, 127, 129, 4095, 4096, 65_537usize] {
            for align in [1, 2, 4, 8, 16, 32usize].iter().copied() {
                let l = layout(size, align);
                let block = A.allocate(l).expect("allocation succeeded");
                assert_eq!(block.len(), size);
                assert_eq!(
                    base(&block) as usize % align,
                    0,
                    "pointer lacks requested alignment for size {} align {}",
                    size,
                    align
                );
                slice_of(&block).fill(0xAB);
                A.deallocate(base_nn(&block), l);
            }
        }
    }
}

#[test]
fn zero_sized_allocations() {
    unsafe {
        for align in [1, 8, 32, 4096usize].iter().copied() {
            let l = layout(0, align);
            let block = A.allocate(l).expect("zero-sized allocation succeeded");
            assert_eq!(block.len(), 0);
            let addr = base(&block) as usize;
            assert_ne!(addr, 0);
            assert_eq!(addr % align, 0, "zero-sized block misaligned for {}", align);
            A.deallocate(base_nn(&block), l);
        }
    }
}

#[test]
fn growing_from_a_zero_sized_allocation() {
    unsafe {
        let z = layout(0, 16);
        let block = A.allocate(z).expect("zero-sized allocation succeeded");
        let new_l = layout(24, 16);
        let grown = A
            .grow(base_nn(&block), z, new_l)
            .expect("growing from zero succeeded");
        assert_eq!(grown.len(), 24);
        assert_eq!(base(&grown) as usize % 16, 0);
        A.deallocate(base_nn(&grown), new_l);
    }
}

#[test]
fn grow_preserves_content_across_size_class() {
    const PATTERN: u8 = 0xA5;
    unsafe {
        // 7 bytes live in the smallest single-digit size class; growing to
        // 9 forces jemalloc past that class boundary.
        let old_l = layout(7, 1);
        let block = A.allocate(old_l).expect("allocation succeeded");
        slice_of(&block).fill(PATTERN);
        let new_l = layout(9, 1);
        let grown = A
            .grow(base_nn(&block), old_l, new_l)
            .expect("growth succeeded");
        assert_eq!(grown.len(), 9);
        let prefix = core::slice::from_raw_parts(base(&grown), 7);
        assert_eq!(prefix, [PATTERN; 7], "original bytes were lost on growth");
        A.deallocate(base_nn(&grown), new_l);
    }
}

#[test]
fn grow_zeroed_zeros_only_the_extension() {
    unsafe {
        let old_l = layout(512, 1);
        let block = A.allocate(old_l).expect("allocation succeeded");
        slice_of(&block).fill(0xCD);
        let new_l = layout(4096, 1);
        let grown = A
            .grow_zeroed(base_nn(&block), old_l, new_l)
            .expect("zeroed growth succeeded");
        assert_eq!(grown.len(), 4096);
        let slice = core::slice::from_raw_parts(base(&grown), 4096);
        assert!(
            slice[..512].iter().all(|&byte| byte == 0xCD),
            "prefix was clobbered"
        );
        assert!(
            slice[512..].iter().all(|&byte| byte == 0),
            "extension was not zeroed"
        );
        A.deallocate(base_nn(&grown), new_l);
    }
}

#[test]
fn shrink_moves_down_in_size_class() {
    unsafe {
        // Pick sizes an order of magnitude apart so that shrinking must land
        // the extent in a clearly smaller size class on success.
        let old_l = layout(1 << 20, 1);
        let block = A.allocate(old_l).expect("large allocation succeeded");
        let original_usable = tikv_jemallocator::usable_size::<u8>(base(&block));
        assert!(original_usable >= (1 << 20));
        slice_of(&block).fill(0x5A);
        let new_l = layout(65_537, 1);
        let shrunk = A
            .shrink(base_nn(&block), old_l, new_l)
            .expect("shrinking succeeded");
        assert_eq!(shrunk.len(), 65_537);
        let usable_after = tikv_jemallocator::usable_size::<u8>(base(&shrunk));
        assert!(
            usable_after < original_usable,
            "shrink did not release any pages: {} vs {}",
            usable_after,
            original_usable
        );
        assert_eq!(*base(&shrunk), 0x5A, "first byte lost on shrink");
        A.deallocate(base_nn(&shrunk), new_l);
    }
}

#[test]
fn grow_with_stronger_alignment_stays_consistent() {
    unsafe {
        let old_l = layout(32, 8);
        let block = A.allocate(old_l).expect("allocation succeeded");
        slice_of(&block).fill(0x13);
        let new_l = layout(64, 64);
        let grown = A
            .grow(base_nn(&block), old_l, new_l)
            .expect("allocation growth succeeded");
        assert_eq!(grown.len(), 64);
        assert_eq!(
            base(&grown) as usize % 64,
            0,
            "grown block does not honor the stronger alignment"
        );
        let slice = core::slice::from_raw_parts(base(&grown), 64);
        assert!(
            slice[..32].iter().all(|&byte| byte == 0x13),
            "prefix was lost"
        );
        A.deallocate(base_nn(&grown), new_l);
    }
}
