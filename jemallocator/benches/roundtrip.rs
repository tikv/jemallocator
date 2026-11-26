//! Benchmarks the cost of the different allocation functions by doing a
//! roundtrip (allocate, deallocate).
//!
//! The benchmarks require nightly Rust and the `nightly_bench` feature.

//! Identifier joining below uses the unstable `macro_metavar_expr_concat`
//! language feature (the official successor to the removed `concat_idents`).
#![feature(macro_metavar_expr_concat)]
#![feature(test)]

extern crate test;

use core::alloc::{Allocator, Layout};
use core::ptr;
use test::Bencher;
use tikv_jemalloc_sys::MALLOCX_ALIGN;
use tikv_jemallocator::Jemalloc;

#[global_allocator]
static A: Jemalloc = Jemalloc;

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
                    let layout = Layout::from_size_align($size, $align).unwrap();
                    let flags = MALLOCX_ALIGN(layout.align());
                    let ptr_addr = jemalloc::mallocx($size, flags);
                    test::black_box(ptr_addr);
                    jemalloc::sdallocx(ptr_addr, $size, flags);
                });
            }

            #[bench]
            fn ${concat(rt_mallocx_nallocx_size_, $size, _align_, $align)}(b: &mut Bencher) {
                b.iter(|| unsafe {
                    use tikv_jemalloc_sys as jemalloc;
                    let layout = Layout::from_size_align($size, $align).unwrap();
                    let flags = MALLOCX_ALIGN(layout.align());
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
                    let layout = Layout::from_size_align($size, $align).unwrap();
                    let flags = MALLOCX_ALIGN(layout.align());
                    let ptr_addr = jemalloc::mallocx($size, flags | jemalloc::MALLOCX_ZERO);
                    test::black_box(ptr_addr);
                    jemalloc::sdallocx(ptr_addr, $size, flags);
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
            #[bench]
            fn ${concat(rt_calloc_size_, $size)}(b: &mut Bencher) {
                b.iter(|| unsafe {
                    use tikv_jemalloc_sys as jemalloc;
                    let ptr_addr = jemalloc::calloc(1, $size);
                    test::black_box(ptr_addr);
                    jemalloc::sdallocx(ptr_addr, $size, 0);
                });
            }

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
