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
    let name = unsafe { Name::<u64>::new(bytes.as_bytes()) }.unwrap();
    assert_eq!(name.as_bytes(), bytes.as_bytes());
    let epoch = name.read().unwrap();
    assert!(epoch > 0);
    name.write(epoch).unwrap();
}

#[test]
fn ctl_rejects_malformed_names() {
    for bytes in [&b""[..], b"\0", b"epoch", b"epoch\0.other\0"] {
        // SAFETY: The constructor rejects these malformed names before returning a handle.
        assert!(unsafe { Name::<u64>::new(bytes) }.is_err());
    }
}
