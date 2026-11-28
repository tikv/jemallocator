use std::alloc::{GlobalAlloc, Layout};
use tikv_jemallocator::Jemalloc;

#[global_allocator]
static A: Jemalloc = Jemalloc;

#[test]
fn smoke() {
    let _a: Vec<i32> = (0..3).collect();
}

/// https://github.com/rust-lang/rust/issues/45955
#[test]
fn overaligned() {
    let align = 16;
    let initial = Layout::from_size_align(8, align).unwrap();
    let resized = Layout::from_size_align(24, align).unwrap();
    let iterations = 100;
    unsafe {
        let pointers: Vec<_> = (0..iterations)
            .map(|_| {
                let ptr = Jemalloc.alloc(initial);
                assert!(!ptr.is_null());
                ptr
            })
            .collect();
        let resized_pointers: Vec<_> = pointers
            .iter()
            .map(|&ptr| {
                assert_eq!(ptr as usize % align, 0);
                let ptr = Jemalloc.realloc(ptr, initial, resized.size());
                assert!(!ptr.is_null());
                ptr
            })
            .collect();
        for &ptr in &resized_pointers {
            assert_eq!(ptr as usize % align, 0);
            Jemalloc.dealloc(ptr, resized);
        }
    }
}

#[test]
fn aligned_allocations_in_distinct_slots() {
    let layout = Layout::from_size_align(24, 16).unwrap();
    unsafe {
        let quantum: usize = tikv_jemalloc_ctl::raw::read(b"arenas.quantum\0").unwrap();
        assert!(tikv_jemalloc_sys::MALLOC_ALIGNMENT <= quantum);
        if let Ok(expected) = std::env::var("JEMALLOC_SYS_TEST_EXPECT_QUANTUM") {
            assert_eq!(quantum, expected.parse::<usize>().unwrap());
        }
        if let Ok(expected) = std::env::var("JEMALLOC_SYS_TEST_EXPECT_ALIGNMENT") {
            assert_eq!(
                tikv_jemalloc_sys::MALLOC_ALIGNMENT,
                expected.parse::<usize>().unwrap()
            );
        }

        let blocks: [_; 16] =
            core::array::from_fn(|_| (Jemalloc.alloc(layout), Jemalloc.alloc_zeroed(layout)));
        for (block, zeroed) in blocks {
            assert!(!block.is_null());
            assert!(!zeroed.is_null());
            assert_eq!(block as usize % layout.align(), 0);
            assert_eq!(zeroed as usize % layout.align(), 0);
            assert_eq!(core::slice::from_raw_parts(zeroed, layout.size()), &[0; 24]);
            Jemalloc.dealloc(block, layout);
            Jemalloc.dealloc(zeroed, layout);
        }
    }
}
