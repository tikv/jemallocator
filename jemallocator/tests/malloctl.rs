use std::alloc::{GlobalAlloc, Layout};
use tikv_jemalloc_ctl::{Access, Name};
use tikv_jemallocator::Jemalloc;

#[global_allocator]
static A: Jemalloc = Jemalloc;

#[test]
fn smoke() {
    let layout = Layout::from_size_align(100, 8).unwrap();
    unsafe {
        let ptr = Jemalloc.alloc(layout);
        assert!(!ptr.is_null());
        Jemalloc.dealloc(ptr, layout);
    }
}

#[test]
fn ctl_get_set() {
    let bytes = String::from("epoch\0");
    // SAFETY: epoch reads and accepts a uint64_t.
    let name = unsafe { Name::<u64>::new(bytes.as_bytes()) };
    assert_eq!(name.as_bytes(), bytes.as_bytes());
    let epoch = name.read().unwrap();
    assert!(epoch > 0);
    name.write(epoch).unwrap();
}

#[test]
#[should_panic(expected = "empty byte-string")]
fn ctl_rejects_empty_name() {
    let _ = unsafe { Name::<u64>::new(b"") };
}

#[test]
#[should_panic(expected = "not NUL-terminated")]
fn ctl_rejects_unterminated_name() {
    let _ = unsafe { Name::<u64>::new(b"epoch") };
}

#[test]
#[should_panic(expected = "interior NUL")]
fn ctl_rejects_name_with_interior_nul() {
    let _ = unsafe { Name::<u64>::new(b"epoch\0.other\0") };
}
