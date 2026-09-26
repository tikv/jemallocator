//! Tests for the freshly stabilized `core::alloc::Allocator` implementation
//! of `Jemalloc`. The target is empty unless the `alloc_trait` feature is
//! enabled, in which case it requires a toolchain that already carries that
//! API (currently the latest nightly). Block base pointers are recovered in
//! `base()` from stable pointer pieces; the still-unstable `as_mut_ptr()` /
//! `as_non_null_ptr()` accessors (rust-lang/rust#74265) are deliberately
//! avoided so these tests compile wherever the trait itself does.

#![cfg(feature = "alloc_trait")]

use core::alloc::{Allocator, Layout};
use core::ptr::NonNull;
use tikv_jemallocator::Jemalloc;

#[global_allocator]
static A: Jemalloc = Jemalloc;

fn layout(size: usize, align: usize) -> Layout {
    Layout::from_size_align(size, align).expect("valid layout")
}

/// Reconstruct the `NonNull<T>` argument expected by the `Allocator` methods.
fn base_nn(block: &NonNull<[u8]>) -> NonNull<u8> {
    // SAFETY: casting the element type of a fat pointer preserves the
    // underlying address along with its full, write-capable provenance;
    // blocks always sit behind a non-null, suitably aligned address.
    unsafe { NonNull::new_unchecked(block.cast::<u8>().as_ptr()) }
}

/// Base data pointer of a block, read through the cast-derived thin pointer.
fn base(block: &NonNull<[u8]>) -> *const u8 {
    base_nn(block).as_ptr()
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
                base_nn(&block).as_ptr().write_bytes(0xAB, block.len());
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
        base_nn(&block).as_ptr().write_bytes(PATTERN, block.len());
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
        base_nn(&block).as_ptr().write_bytes(0xCD, block.len());
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
        base_nn(&block).as_ptr().write_bytes(0x5A, block.len());
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
        base_nn(&block).as_ptr().write_bytes(0x13, block.len());
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

/// Regression guard: shrinking into a zero-sized layout whose required
/// alignment far exceeds what any backing allocation can provide must not
/// route through the raw allocation primitives, which reject zero-size
/// requests outright.
#[test]
fn shrink_to_zero_with_exotic_alignment_stays_consistent() {
    unsafe {
        let old_l = layout(8, 8);
        let block = A.allocate(old_l).expect("allocation succeeded");
        base_nn(&block).as_ptr().write_bytes(0x5A, block.len());
        let new_l = layout(0, 1 << (core::mem::size_of::<usize>() * 8 - 2));
        let shrunk = A
            .shrink(base_nn(&block), old_l, new_l.clone())
            .expect("shrinking to zero succeeded");
        assert_eq!(shrunk.len(), 0);
        let addr = base(&shrunk) as usize;
        assert_ne!(addr, 0);
        assert_eq!(
            addr % new_l.align(),
            0,
            "zero block lacks its promised alignment"
        );
        A.deallocate(base_nn(&shrunk), new_l);
    }
}

/// Version-sensitive regression guards, see the routing notes on
/// `resize_blocks`: jemalloc accounts allocations by their *quantized* size
/// class, and its sized-free fastpath trusts the hint-derived class instead
/// of looking up the record. Two layouts that land in the same class are
/// interchangeable across allocate/shrink/deallocate even though their
/// numeric sizes differ -- but the premise must survive a jemalloc bump.
///
/// The class-table asserts below fail loudly at build time if the small
/// size table ever shifts, forcing a human to re-validate that contract
/// before anything silently desyncs again; the soak loops exercise exactly
/// the shapes that corrupted heap metadata when hints crossed a boundary.
#[test]
fn same_class_resize_hints_stay_consistent_across_sized_frees() {
    // Pin the class table assumptions first: 15 -> 9 must share a class
    // (otherwise no real pointer would ever be reused in place), while
    // 2000 -> 1000 must not (the relocation branch depends on seeing it).
    let same_16 = unsafe {
        (
            tikv_jemalloc_sys::nallocx(15, 0),
            tikv_jemalloc_sys::nallocx(9, 0),
        )
    };
    assert_eq!(
        same_16.0, same_16.1,
        "15B and 9B drifted apart in size classes"
    );
    let cross = unsafe {
        (
            tikv_jemalloc_sys::nallocx(2000, 0),
            tikv_jemalloc_sys::nallocx(1000, 0),
        )
    };
    assert_ne!(
        cross.0, cross.1,
        "2000B and 1000B unexpectedly share a class"
    );

    unsafe {
        for _ in 0..(1 << 18) {
            // Request 15 bytes, legally shrink toward the 9-byte layout,
            // then free under 9: jemalloc never received 9 as a request,
            // yet the freed hint shares the recorded class.
            let old_l = layout(15, 1);
            let block = A.allocate(old_l).expect("allocation succeeded");
            base_nn(&block).as_ptr().write_bytes(0x5A, block.len());
            let shrunk = A
                .shrink(base_nn(&block), old_l, layout(9, 1))
                .expect("shrink stayed valid");
            assert_eq!(shrunk.len(), 9);
            A.deallocate(base_nn(&shrunk), layout(9, 1));

            // Equal-size relaxations change nothing recorded: same bytes,
            // weaker alignment.
            let big_l = layout(64, 64);
            let z = A.allocate_zeroed(big_l).expect("allocation succeeded");
            let relaxed = A
                .grow(base_nn(&z), big_l, layout(64, 8))
                .expect("alignment relaxation succeeded");
            assert_eq!(relaxed.len(), 64);
            A.deallocate(base_nn(&relaxed), layout(64, 8));
        }
    }
}

#[test]
fn repeated_cross_class_shrink_stays_safe() {
    // Regression guard: repeatedly shrinking across size classes while
    // freeing under the shrunken layout must not desynchronize jemalloc's
    // size bookkeeping. A stale record plus an off-bucket sized free used to
    // route pointers into the wrong caches and corrupt heap metadata.
    unsafe {
        for _ in 0..2_000 {
            let wide_l = layout(2000, 1);
            let block = A.allocate(wide_l).expect("allocation succeeded");
            base_nn(&block).as_ptr().write_bytes(0x77, block.len());
            let narrow_l = layout(1000, 1);
            let shrunk = A
                .shrink(base_nn(&block), wide_l, narrow_l)
                .expect("shrinking succeeded");
            assert_eq!(shrunk.len(), 1000);
            assert_eq!(*base(&shrunk), 0x77);
            A.deallocate(base_nn(&shrunk), narrow_l);
        }
    }
}

#[test]
fn repeated_large_shrink_with_layout_free_stays_safe() {
    unsafe {
        for _ in 0..256 {
            let big_l = layout(1 << 20, 1);
            let block = A.allocate(big_l).expect("large allocation succeeded");
            *base_nn(&block).as_ptr() = 0x3E;
            let small_l = layout(65_537, 1);
            let shrunk = A
                .shrink(base_nn(&block), big_l, small_l)
                .expect("large shrink succeeded");
            assert_eq!(shrunk.len(), 65_537);
            assert_eq!(*base(&shrunk), 0x3E);
            A.deallocate(base_nn(&shrunk), small_l);
        }
    }
}

#[test]
fn large_growth_preserves_prefix_and_stays_consistent() {
    unsafe {
        for _ in 0..256 {
            let old_l = layout(65_537, 1);
            let block = A.allocate(old_l).expect("large allocation succeeded");
            base_nn(&block).as_ptr().write_bytes(0x1F, block.len());
            let new_l = layout(1 << 20, 1);
            let grown = A
                .grow(base_nn(&block), old_l, new_l)
                .expect("growth succeeded");
            assert_eq!(grown.len(), 1 << 20);
            let prefix = core::slice::from_raw_parts(base(&grown), 65_537);
            assert!(
                prefix.iter().all(|&byte| byte == 0x1F),
                "prefix was lost on growth"
            );
            A.deallocate(base_nn(&grown), new_l);
        }
    }
}

/// The grow/shrink settle gate keys off the extent-managed regime's floor
/// (`LARGE_REGIME_MIN_USIZE`, 32 KiB for the bundled build); this pins that
/// floor against the live size-class table so a jemalloc bump that shifts
/// the table fails loudly instead of silently re-enabling futile settle
/// attempts -- or disabling viable ones.
#[test]
fn resize_regime_floor_stays_pinned() {
    const MIN_LARGE: usize = 32_768; // mirrors the crate-private constant
    unsafe {
        assert_eq!(
            tikv_jemalloc_sys::nallocx(MIN_LARGE, 0),
            MIN_LARGE,
            "regime floor no longer its own extent class"
        );
        assert!(
            tikv_jemalloc_sys::nallocx(MIN_LARGE + 1, 0) > MIN_LARGE,
            "requests past the floor must not stay in its class"
        );
        assert!(
            tikv_jemalloc_sys::nallocx(MIN_LARGE - 16, 0) <= MIN_LARGE,
            "requests below the floor may not overshoot into higher classes"
        );
        assert!(
            tikv_jemalloc_sys::nallocx(24_000, 0) < MIN_LARGE,
            "mixed-regime soak endpoint unexpectedly extent-managed"
        );
    }
}

/// Cross-class resizes under the settle gate must behave identically to
/// plain relocate-and-copy wherever the gate skips an attempt: contents
/// preserved through many alternating grow/shrink cycles, records kept
/// consistent with the layout each block is later freed under. The pairs
/// cover the three regime combinations the gate distinguishes: slab/slab,
/// slab/extent, and extent/extent (where settling stays attempted).
#[test]
fn cross_class_resizes_stay_consistent_under_settle_gate() {
    // Pin the intended class relationships up front so a drifting table
    // fails loudly instead of mislabeling which gate branch is exercised.
    unsafe {
        assert_ne!(
            tikv_jemalloc_sys::nallocx(96, 0),
            tikv_jemalloc_sys::nallocx(192, 0)
        );
        assert!(
            tikv_jemalloc_sys::nallocx(24_000, 0) < 32_768
                && tikv_jemalloc_sys::nallocx(30_000, 0) >= 32_768,
            "mixed-regime pair no longer straddles the regime boundary"
        );
        assert!(
            tikv_jemalloc_sys::nallocx(40_000, 0) >= 32_768
                && tikv_jemalloc_sys::nallocx(80_000, 0) >= 32_768
                && tikv_jemalloc_sys::nallocx(40_000, 0) != tikv_jemalloc_sys::nallocx(80_000, 0),
            "extent pair drifted out of the settled regime"
        );
    }
    for &(lo, hi, cycles) in &[
        (96usize, 192, 1_024),
        (24_000, 30_000, 256),
        (40_000, 80_000, 128),
    ] {
        let low_l = layout(lo, 8);
        let high_l = layout(hi, 8);
        unsafe {
            let mut block = A.allocate(low_l.clone()).expect("allocation succeeded");
            base_nn(&block).as_ptr().write_bytes(0x4C, lo);
            for i in 0..cycles {
                if i % 2 == 0 {
                    let grown = A
                        .grow(base_nn(&block), low_l.clone(), high_l.clone())
                        .expect("growth succeeded");
                    let rec = tikv_jemalloc_sys::sallocx(base(&grown) as *const _, 0);
                    assert_eq!(
                        rec,
                        tikv_jemalloc_sys::nallocx(hi, 0),
                        "record drifted after grow to {} bytes",
                        hi
                    );
                    let prefix = core::slice::from_raw_parts(base(&grown), lo);
                    assert!(
                        prefix.iter().all(|&byte| byte == 0x4C),
                        "prefix lost growing {} -> {} bytes",
                        lo,
                        hi
                    );
                    block = grown;
                } else {
                    let shrunk = A
                        .shrink(base_nn(&block), high_l.clone(), low_l.clone())
                        .expect("shrink succeeded");
                    let rec = tikv_jemalloc_sys::sallocx(base(&shrunk) as *const _, 0);
                    assert_eq!(
                        rec,
                        tikv_jemalloc_sys::nallocx(lo, 0),
                        "record drifted after shrink to {} bytes",
                        lo
                    );
                    let prefix = core::slice::from_raw_parts(base(&shrunk), lo);
                    assert!(
                        prefix.iter().all(|&byte| byte == 0x4C),
                        "prefix lost shrinking {} -> {} bytes",
                        hi,
                        lo
                    );
                    block = shrunk;
                }
            }
            // Final release routes through a sized free of the exact layout
            // the block currently serves.
            A.deallocate(base_nn(&block), low_l.clone());
        }
    }
}
