//! Tests for the `core::alloc::Allocator` implementation of `Jemalloc`, which
//! requires the `alloc_trait` feature and Rust 1.100 or later.

#![cfg(feature = "alloc_trait")]

use core::alloc::{Allocator, Layout};
use core::ptr::NonNull;
use tikv_jemalloc_ctl::raw;
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
        for &size in &[
            1,
            3,
            7,
            8,
            16,
            24,
            40,
            56,
            127,
            129,
            4095,
            4096,
            65_537usize,
        ] {
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
fn aligned_blocks_from_distinct_slots() {
    let layout = layout(24, 16);
    let blocks: [_; 16] = core::array::from_fn(|_| {
        (
            A.allocate(layout).unwrap(),
            A.allocate_zeroed(layout).unwrap(),
        )
    });

    for (block, zeroed) in blocks {
        assert_eq!(base(&block) as usize % layout.align(), 0);
        assert_eq!(base(&zeroed) as usize % layout.align(), 0);
        unsafe {
            assert!(core::slice::from_raw_parts(base(&zeroed), layout.size())
                .iter()
                .all(|&byte| byte == 0));
            A.deallocate(base_nn(&block), layout);
            A.deallocate(base_nn(&zeroed), layout);
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
fn grow_zeroed_within_size_class() {
    let old = layout(9, 1);
    let new = layout(16, 1);
    unsafe {
        let block = A.allocate(new).unwrap();
        let ptr = base_nn(&block);
        let original_address = ptr.as_ptr() as usize;
        // Fill the entire block before shrinking so the in-place growth
        // exposes nonzero bytes unless it explicitly clears the tail.
        ptr.as_ptr().write_bytes(0xCD, new.size());
        let shrunk = A.shrink(ptr, new, old).unwrap();
        assert_eq!(base(&shrunk) as usize, original_address);
        let grown = A.grow_zeroed(base_nn(&shrunk), old, new).unwrap();
        assert_eq!(base(&grown) as usize, original_address);
        assert_eq!(grown.len(), new.size());
        let bytes = core::slice::from_raw_parts(base(&grown), grown.len());
        assert_eq!(&bytes[..old.size()], &[0xCD; 9]);
        assert_eq!(&bytes[old.size()..], &[0; 7]);
        A.deallocate(base_nn(&grown), new);
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
        let slice = core::slice::from_raw_parts(base(&grown), old_l.size());
        assert!(slice.iter().all(|&byte| byte == 0x13), "prefix was lost");
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

// A live block can be released with its current layout after the size or
// alignment changes in place.
#[test]
fn same_class_resizes_can_be_freed_with_the_current_layout() {
    unsafe {
        assert_eq!(
            tikv_jemalloc_sys::nallocx(15, 0),
            tikv_jemalloc_sys::nallocx(9, 0),
        );
    }

    unsafe {
        for _ in 0..(1 << 18) {
            let old_l = layout(15, 1);
            let block = A.allocate(old_l).expect("allocation succeeded");
            base_nn(&block).as_ptr().write_bytes(0x5A, block.len());
            let address = base(&block) as usize;
            let shrunk = A
                .shrink(base_nn(&block), old_l, layout(9, 1))
                .expect("shrink stayed valid");
            assert_eq!(shrunk.len(), 9);
            assert_eq!(base(&shrunk) as usize, address);
            A.deallocate(base_nn(&shrunk), layout(9, 1));

            let big_l = layout(64, 64);
            let z = A.allocate_zeroed(big_l).expect("allocation succeeded");
            let address = base(&z) as usize;
            let relaxed = A
                .grow(base_nn(&z), big_l, layout(64, 8))
                .expect("alignment relaxation succeeded");
            assert_eq!(relaxed.len(), 64);
            assert_eq!(base(&relaxed) as usize, address);
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

fn first_extent_size() -> usize {
    unsafe { raw::read(b"arenas.lextent.0.size\0") }.unwrap()
}

#[test]
fn shrink_to_first_extent_class_preserves_contents() {
    let narrow = layout(first_extent_size(), 1);
    let wide = layout(narrow.size() * 2, 1);
    unsafe {
        let block = A.allocate(wide).unwrap();
        base_nn(&block).as_ptr().write_bytes(0x64, narrow.size());
        let shrunk = A.shrink(base_nn(&block), wide, narrow).unwrap();
        assert_eq!(shrunk.len(), narrow.size());
        let contents = core::slice::from_raw_parts(base(&shrunk), narrow.size());
        assert!(contents.iter().all(|&byte| byte == 0x64));
        A.deallocate(base_nn(&shrunk), narrow);
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
    let first_extent = first_extent_size();
    let nbins: libc::c_uint = unsafe { raw::read(b"arenas.nbins\0") }.unwrap();
    let mut bin_size_mib = [0; 4];
    raw::name_to_mib(b"arenas.bin.0.size\0", &mut bin_size_mib).unwrap();
    bin_size_mib[2] = nbins as usize - 1;
    let last_slab: usize = unsafe { raw::read_mib(&bin_size_mib) }.unwrap();
    assert!(last_slab < first_extent);

    for &(lo, hi, cycles) in &[
        (96usize, 192, 1_024),
        (last_slab, first_extent, 256),
        (first_extent, first_extent * 2, 128),
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
            A.deallocate(base_nn(&block), low_l.clone());
        }
    }
}
