use tikv_jemalloc_ctl::{epoch, Access};

#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

// Refresh the allocator's global epoch in a separate test process so the
// unit tests can compare stats for the same epoch.
#[test]
fn read_write_update() {
    let initial = epoch::read().unwrap();
    epoch::write(0).unwrap();
    let written = epoch::read().unwrap();
    assert_eq!(written, initial.wrapping_add(1));
    let updated = epoch::update(0).unwrap();
    assert_eq!(updated, written.wrapping_add(1));

    let mib = epoch::mib().unwrap();
    assert_eq!(mib.read().unwrap(), updated);
    mib.write(0).unwrap();
    let mib_written = mib.read().unwrap();
    assert_eq!(mib_written, updated.wrapping_add(1));
    let mib_updated = mib.update(0).unwrap();
    assert_eq!(mib_updated, mib_written.wrapping_add(1));

    let name = epoch::name();
    let typed_mib = name.mib().unwrap();
    assert_eq!(name.read().unwrap(), mib_updated);
    name.write(0).unwrap();
    let name_written = typed_mib.read().unwrap();
    assert_eq!(name_written, mib_updated.wrapping_add(1));
    typed_mib.write(0).unwrap();
    assert_eq!(name.read().unwrap(), name_written.wrapping_add(1));
}
