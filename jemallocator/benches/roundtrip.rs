//! Benchmarks the cost of the different allocation functions by doing a
//! roundtrip (allocate, deallocate).
//!
//! Empty unless the `alloc_trait` feature is enabled, which requires a
//! toolchain that already carries the freshly stabilized `Allocator` API
//! (currently the latest nightly). Block pointers are recovered through
//! stable element casts, keeping the benchmarks free of the not-yet-stable
//! block accessors.

//! Identifier joining below uses the unstable `macro_metavar_expr_concat`
//! language feature (the official successor to the removed `concat_idents`).
#![feature(macro_metavar_expr_concat)]
#![feature(test)]
#![cfg(feature = "alloc_trait")]

extern crate test;

use core::alloc::{Allocator, Layout};
use core::ptr;
use libc::c_int;
use test::Bencher;
use tikv_jemalloc_sys::MALLOCX_ALIGN;
use tikv_jemallocator::Jemalloc;

#[global_allocator]
static A: Jemalloc = Jemalloc;

// FIXME: replace with jemallocator::layout_to_flags
#[cfg(any(target_arch = "arm", target_arch = "mips", target_arch = "powerpc"))]
const MIN_ALIGN: usize = 8;
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "powerpc64",
    target_arch = "loongarch64",
    target_arch = "mips64",
    target_arch = "riscv64",
    target_arch = "s390x",
    target_arch = "sparc64"
))]
const MIN_ALIGN: usize = 16;

fn layout_to_flags(layout: &Layout) -> c_int {
    if layout.align() <= MIN_ALIGN && layout.align() <= layout.size() {
        0
    } else {
        MALLOCX_ALIGN(layout.align())
    }
}

fn base_nn(block: &ptr::NonNull<[u8]>) -> ptr::NonNull<u8> {
    // SAFETY: casting the element type preserves the block's own address
    // along with its full, write-capable provenance; see the allocator_api
    // tests for notes on stable thin-pointer recovery.
    unsafe { ptr::NonNull::new_unchecked(block.cast::<u8>().as_ptr()) }
}

fn base(block: &ptr::NonNull<[u8]>) -> *const u8 {
    base_nn(block).as_ptr()
}

macro_rules! rt {
    ($size:literal, $align:literal) => {
            #[bench]
            fn ${concat(rt_mallocx_size_, $size, _align_, $align)}(b: &mut Bencher) {
                b.iter(|| unsafe {
                    use tikv_jemalloc_sys as jemalloc;
                    let flags = layout_to_flags(&Layout::from_size_align($size, $align).unwrap());
                    let ptr_addr = jemalloc::mallocx($size, flags);
                    test::black_box(ptr_addr);
                    jemalloc::sdallocx(ptr_addr, $size, flags);
                });
            }

            #[bench]
            fn ${concat(rt_mallocx_nallocx_size_, $size, _align_, $align)}(b: &mut Bencher) {
                b.iter(|| unsafe {
                    use tikv_jemalloc_sys as jemalloc;
                    let flags = layout_to_flags(&Layout::from_size_align($size, $align).unwrap());
                    let ptr_addr = jemalloc::mallocx($size, flags);
                    test::black_box(ptr_addr);
                    let rsz = jemalloc::nallocx($size, flags);
                    test::black_box(rsz);
                    jemalloc::sdallocx(ptr_addr, rsz, flags);
                });
            }

            #[bench]
            fn ${concat(rt_allocate_size_, $size, _align_, $align)}(b: &mut Bencher) {
                b.iter(|| unsafe {
                    let layout = Layout::from_size_align($size, $align).unwrap();
                    let block = Jemalloc.allocate(layout.clone()).unwrap();
                    test::black_box(block);
                    Jemalloc.deallocate(base_nn(&block), layout);
                });
            }

            #[bench]
            fn ${concat(rt_mallocx_zeroed_size_, $size, _align_, $align)}(b: &mut Bencher) {
                b.iter(|| unsafe {
                    use tikv_jemalloc_sys as jemalloc;
                    let flags = layout_to_flags(&Layout::from_size_align($size, $align).unwrap());
                    let ptr_addr = jemalloc::mallocx($size, flags | jemalloc::MALLOCX_ZERO);
                    test::black_box(ptr_addr);
                    jemalloc::sdallocx(ptr_addr, $size, flags);
                });
            }

            #[bench]
            fn ${concat(rt_calloc_size_, $size, _align_, $align)}(b: &mut Bencher) {
                b.iter(|| unsafe {
                    use tikv_jemalloc_sys as jemalloc;
                    let flags = layout_to_flags(&Layout::from_size_align($size, $align).unwrap());
                    test::black_box(flags);
                    let ptr_addr = jemalloc::calloc(1, $size);
                    test::black_box(ptr_addr);
                    jemalloc::sdallocx(ptr_addr, $size, 0);
                });
            }

            #[bench]
            fn ${concat(rt_grow_naive_size_, $size, _align_, $align)}(b: &mut Bencher) {
                b.iter(|| unsafe {
                    let layout = Layout::from_size_align($size, $align).unwrap();
                    let block = Jemalloc.allocate(layout.clone()).unwrap();
                    test::black_box(block);

                    // naive realloc: allocate a fresh block, copy over, free the
                    // original.
                    let new_layout = Layout::from_size_align(2 * $size, $align).unwrap();
                    let block = {
                        let new_block = Jemalloc.allocate(new_layout.clone()).unwrap();
                        ptr::copy_nonoverlapping(
                            base(&block) as *mut u8,
                            base(&new_block) as *mut u8,
                            layout.size(),
                        );
                        Jemalloc.deallocate(base_nn(&block), layout);
                        new_block
                    };
                    test::black_box(block);

                    Jemalloc.deallocate(base_nn(&block), new_layout);
                });
            }

            #[bench]
            fn ${concat(rt_grow_size_, $size, _align_, $align)}(b: &mut Bencher) {
                b.iter(|| unsafe {
                    let layout = Layout::from_size_align($size, $align).unwrap();
                    let block = Jemalloc.allocate(layout.clone()).unwrap();
                    test::black_box(block);

                    let new_layout = Layout::from_size_align(2 * $size, $align).unwrap();
                    let grown = Jemalloc
                        .grow(base_nn(&block), layout, new_layout.clone())
                        .unwrap();
                    test::black_box(grown);

                    Jemalloc.deallocate(base_nn(&grown), new_layout);
                });
            }

            #[bench]
            fn ${concat(rt_grow_zeroed_size_, $size, _align_, $align)}(b: &mut Bencher) {
                b.iter(|| unsafe {
                    let layout = Layout::from_size_align($size, $align).unwrap();
                    let block = Jemalloc.allocate_zeroed(layout.clone()).unwrap();
                    test::black_box(block);

                    let new_layout = Layout::from_size_align(2 * $size, $align).unwrap();
                    let grown = Jemalloc
                        .grow_zeroed(base_nn(&block), layout, new_layout.clone())
                        .unwrap();
                    test::black_box(grown);

                    Jemalloc.deallocate(base_nn(&grown), new_layout);
                });
            }

            #[bench]
            fn ${concat(rt_shrink_size_, $size, _align_, $align)}(b: &mut Bencher) {
                b.iter(|| unsafe {
                    let wide_layout = Layout::from_size_align(2 * $size, $align).unwrap();
                    let block = Jemalloc.allocate(wide_layout.clone()).unwrap();
                    test::black_box(block);

                    let small_layout = Layout::from_size_align($size, $align).unwrap();
                    let shrunk = Jemalloc
                        .shrink(base_nn(&block), wide_layout, small_layout.clone())
                        .unwrap();
                    test::black_box(shrunk);

                    Jemalloc.deallocate(base_nn(&shrunk), small_layout);
                });
            }

    };
    ([$($size:literal),*]) => {
        $(
            rt!($size, 1);
            rt!($size, 2);
            rt!($size, 4);
            rt!($size, 8);
            rt!($size, 16);
            rt!($size, 32);
        )*
    }
}

// Powers of two
mod pow2 {
    use super::*;

    rt!([
        1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768, 65536, 131072,
        4194304
    ]);
}

mod even {
    use super::*;

    rt!([10, 100, 1000, 10000, 100000, 1000000]);
}

mod odd {
    use super::*;
    rt!([9, 99, 999, 9999, 99999, 999999]);
}

mod primes {
    use super::*;
    rt!([
        3, 7, 13, 17, 31, 61, 96, 127, 257, 509, 1021, 2039, 4093, 8191, 16381, 32749, 65537,
        131071, 4194301
    ]);
}
