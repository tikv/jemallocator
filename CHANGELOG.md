# 0.5.4 - 2023-07-22

- Add disable_initial_exec_tls feature for jemalloc-ctl (#59)
- Fix definition of `c_bool` for non-MSVC targets (#54)
- Add `disable_cache_oblivious` feature (#51)
- Add loongarch64 support (#42)

# jemalloc-sys 0.5.3 - 2023-02-03

- Remove fs-extra dependency (#47)

# jemalloc-sys 0.5.2 - 2022-09-29

- Fix build on riscv64gc-unknown-linux-gnu (#40)

# jemalloc-sys 0.5.1 - 2022-06-22

- Backport support for NetBSD (#31)
- Watch environment variable change in build script (#31)

# 0.5.0 - 2022-05-19

- Update jemalloc to 5.3.0 (#23)

# 0.4.3 - 2022-02-21

- Added riscv64 support (#14)

# 0.4.2 - 2021-08-09

- Fixed prof not working under certain condition (#9) (#12)
- Updated paste to 1 (#11)

# 0.4.1 - 2020-11-16

- Updated jemalloc to fix deadlock during initialization
- Fixed failure of generating docs on release version

# 0.4.0 - 2020-07-21

- Forked from jemallocator master
- Upgraded jemalloc to 5.2.1 (#1)
- Fixed wrong version in generated C header (#1)
- Upgraded project to 2018 edition (#2)
