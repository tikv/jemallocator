use tikv_jemalloc_ctl::{opt, Access, Name};

#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[test]
fn read_boolean_option_by_name_and_mib() {
    let abort = opt::abort::read().unwrap();
    assert_eq!(opt::abort::mib().unwrap().read().unwrap(), abort);
}

#[test]
fn roundtrip_thread_tcache_by_name_and_mib() {
    // SAFETY: jemalloc defines this read/write key as a thread-local bool.
    let name =
        unsafe { Name::<bool>::new(b"thread.tcache.enabled\0") }.unwrap();
    let mib = name.mib().unwrap();
    let initial = name.read().unwrap();

    name.write(!initial).unwrap();
    assert_eq!(mib.read().unwrap(), !initial);
    assert_eq!(name.update(initial).unwrap(), !initial);
    assert_eq!(mib.update(!initial).unwrap(), initial);
    assert_eq!(name.read().unwrap(), !initial);
    mib.write(initial).unwrap();
    assert_eq!(name.read().unwrap(), initial);
}
