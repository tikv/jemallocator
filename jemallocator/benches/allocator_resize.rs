//! End-to-end benchmarks for `Jemalloc`'s `grow`/`shrink`: each measured
//! operation is a full trait-level resize (class probing, settle judgement,
//! and any relocation plus copy it entails), driven through the real
//! implementation on a live block toggling between two requested sizes --
//! the shape collection capacity churn takes in practice.
//!
//! The point of keeping these in-tree is bookkeeping: every future change
//! to the resize paths lands on the same instrument, so an optimization is
//! judged by whole-operation deltas instead of per-call intuition.
//!
//! Empty unless the `alloc_trait` feature is enabled, which requires a
//! toolchain that already carries the freshly stabilized `Allocator` API
//! (currently the latest nightly). Block pointers are recovered through
//! stable element casts, matching the rest of the allocator API surface.

#![feature(test)]
#![cfg(feature = "alloc_trait")]

extern crate test;

use core::alloc::{Allocator, Layout};
use core::ptr;
use test::{black_box, Bencher};
use tikv_jemallocator::Jemalloc;

#[global_allocator]
static A: Jemalloc = Jemalloc;

fn layout(size: usize, align: usize) -> Layout {
    Layout::from_size_align(size, align).expect("valid layout")
}

/// Reconstruct the `NonNull<T>` argument expected by the `Allocator` methods.
fn base_nn(block: &ptr::NonNull<[u8]>) -> ptr::NonNull<u8> {
    // SAFETY: casting the element type preserves the block's own address
    // along with its full, write-capable provenance.
    unsafe { ptr::NonNull::new_unchecked(block.cast::<u8>().as_ptr()) }
}

struct ToggleState {
    block: ptr::NonNull<[u8]>,
    // `false` => currently serving the small layout.
    grew: bool,
}

impl ToggleState {
    fn one_resize(&mut self, small: Layout, big: Layout) -> ptr::NonNull<[u8]> {
        unsafe {
            if self.grew {
                self.block = A
                    .shrink(base_nn(&self.block), big, small)
                    .expect("shrink stayed valid");
            } else {
                self.block = A
                    .grow(base_nn(&self.block), small, big)
                    .expect("grow stayed valid");
            }
            self.grew = !self.grew;
        }
        self.block
    }
}

fn assert_classes(a: usize, b: usize, relation: &str) {
    // Pin the intended size-class relationship of each workload up front so
    // a silently drifting table fails loudly instead of mislabeling runs.
    let (qa, qb) = (unsafe { tikv_jemalloc_sys::nallocx(a, 0) }, unsafe {
        tikv_jemalloc_sys::nallocx(b, 0)
    });
    let ok = match relation {
        "differ" => qa != qb,
        "same" => qa == qb,
        _ => panic!("unknown relation {}", relation),
    };
    assert!(
        ok,
        "{} bytes vs {} bytes: resolved classes ({}, {}) do not {} as documented",
        a, b, qa, qb, relation
    );
}

/// Cross-class small resize: every operation must relocate; this is the hot
/// path where a provably futile in-place settle attempt costs real cycles.
#[bench]
fn resize_cross_class_64_128(b: &mut Bencher) {
    assert_classes(64, 128, "differ");
    let small = layout(64, 8);
    let big = layout(128, 8);
    let mut st = ToggleState {
        block: A.allocate(small.clone()).expect("allocation succeeded"),
        grew: false,
    };
    let mut sink: u64 = 0;
    b.iter(|| {
        let nb = st.one_resize(small.clone(), big.clone());
        sink ^= nb.cast::<u8>().as_ptr() as u64;
        if sink == u64::MAX {
            black_box(sink);
        }
    });
}

/// Same-bucket resize: the pointer is retained outright; a regression-free
/// control proving cheap stays cheap.
#[bench]
fn resize_same_bucket_232_256(b: &mut Bencher) {
    assert_classes(232, 256, "same");
    let small = layout(232, 8);
    let big = layout(256, 8);
    let mut st = ToggleState {
        block: A.allocate(small.clone()).expect("allocation succeeded"),
        grew: false,
    };
    let mut sink: u64 = 0;
    b.iter(|| {
        let nb = st.one_resize(small.clone(), big.clone());
        sink ^= nb.cast::<u8>().as_ptr() as u64;
        if sink == u64::MAX {
            black_box(sink);
        }
    });
}

/// Cross-class resize around a kilobyte: relocation dominated by the copy;
/// shows how much of the previous overhead lived outside the copy itself.
#[bench]
fn resize_cross_class_1024_1536(b: &mut Bencher) {
    assert_classes(1024, 1536, "differ");
    let small = layout(1024, 8);
    let big = layout(1536, 8);
    let mut st = ToggleState {
        block: A.allocate(small.clone()).expect("allocation succeeded"),
        grew: false,
    };
    let mut sink: u64 = 0;
    b.iter(|| {
        let nb = st.one_resize(small.clone(), big.clone());
        sink ^= nb.cast::<u8>().as_ptr() as u64;
        if sink == u64::MAX {
            black_box(sink);
        }
    });
}

/// Cross-class resize with both endpoints in the extent-managed regime,
/// where in-place settling is genuinely viable: a control proving the
/// mechanism that does get exercised stays unchanged.
#[bench]
fn resize_large_inplace_262144_311296(b: &mut Bencher) {
    assert_classes(262_144, 311_296, "differ");
    let small = layout(262_144, 8);
    let big = layout(311_296, 8);
    let mut st = ToggleState {
        block: A.allocate(small.clone()).expect("allocation succeeded"),
        grew: false,
    };
    let mut sink: u64 = 0;
    b.iter(|| {
        let nb = st.one_resize(small.clone(), big.clone());
        sink ^= nb.cast::<u8>().as_ptr() as u64;
        if sink == u64::MAX {
            black_box(sink);
        }
    });
}
