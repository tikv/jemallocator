# Unreleased

- tikv-jemallocator: implement the freshly stabilized `core::alloc::Allocator`
  API behind the existing `alloc_trait` feature, replacing the removed
  nightly-only `Alloc`/`Excess` implementation. Enabling the feature requires
  a toolchain that already carries the API (currently only available on
  nightly); older toolchains should simply leave it off. The implementation
  supports zero-sized layouts per the new contract, and grow/shrink stay
  in-place whenever the target layout maps onto the same jemalloc size bucket
  or jemalloc can refresh the allocation in place, falling back to moving
  otherwise -- keeping the per-allocation size record consistent with the
  layout under which blocks eventually get freed. The rewritten tests and
  benchmarks also exercise the full block lifecycle; they recover block
  pointers in small stable-helper functions, sidestepping the not-yet-stable
  block accessors (`as_non_null_ptr`/`as_mut_ptr`), and CI exercises the
  feature on the latest nightly (the `ALLOC_TRAIT_TESTS` hook in `ci/run.sh`
  plus the `test_bench` job).
- tikv-jemallocator: grow/shrink skip jemalloc's in-place settle attempts
  wherever the size-class table makes them provably futile -- any resize in
  which an endpoint resolves slab-managed can never settle across classes,
  so those resizes go straight to relocation instead (measured end-to-end
  delta roughly -6 ns/resize on sub-4KiB and 1 KiB-class churn). End-to-end
  resize benchmarks (`benches/allocator_resize.rs`, nightly-gated with the
  rest of the allocator API surface) track whole-operation costs across
  regime combinations so future tuning lands on a fixed instrument.
- tikv-jemalloc-sys: expose the jemalloc 5.4.0 extent-allocation hook flags
  `EXTENT_ALLOC_FLAG_PINNED` and `EXTENT_ALLOC_FLAG_MASK`, documenting the
  low-bit protocol of the pointer returned from `extent_alloc_t` hooks.
- tikv-jemalloc-ctl: expose the `stats.pinned` mallctl added in jemalloc 5.4.0
  via `stats::pinned`.
- Remove the dependency on the unmaintained `paste` crate:
  - `tikv-jemalloc-ctl`: key-generation macros no longer derive
    identifiers at compile time; `option!` takes the companion MIB
    type name explicitly (`epoch epoch_mib[ str: ..., non_str: 1 ]
    => u64 |`) together with the generated test name (`test:
    epoch_read_write_update_test |`). Generated names and public
    API are unchanged.
  - the nightly-gated allocator benchmarks now use std's own
    successor mechanism, the unstable `macro_metavar_expr_concat`
    feature (`${ concat(prefix, $size, suffix) }`), replacing both
    `paste` and any in-repo replacement crate.
- jemalloc-ctl: expose `prof.active`, `prof.lg_sample`, and `prof.reset` via
  `profiling::{prof_active, lg_sample, prof_reset}`
- jemalloc-ctl: expose jemalloc's experimental sample hooks under the
  `profiling` feature (`set_prof_sample_hook`, `set_prof_sample_free_hook`,
  `set_prof_backtrace_hook`, `noop_prof_backtrace_hook`, and the `Prof*Hook`
  types)

# 0.7.0 - 2026-05-25

- Reverse order of MAKEFLAGS priority (#152)
- Define ALIGNOF_MAX_ALIGN_T for riscv32 (#153)
- Remove build directory once build of `jemalloc-sys` finishes (#119)
- Fix cross-compile for tier-3 riscv64a23 target (#141)
- sys: support *-windows-gnullvm targets (#150)
- Propagate LDFLAGS, if present (#155)
- jemalloc-ctl: fix invalid update implementation
- add new free ffi
- Update jemalloc to 5.3.1 (#161)
- Add profiling_libunwind feature (#159)
- passthrough cc env/args using native cc features (#158)

# 0.6.1 - 2025-10-15

- Fix compiler and clippy warnings (#105)
- Add feature `disable_cache_oblivious` to jemallocator re-exports (#104)
- Document `JEMALLOC_OVERRIDE` (#107)
- Harden `strerror_r` function detection (#117)
- Respect jobserver set by Cargo (#120)
- Make unprefixed consistently override the system allocator (#109)
  - Adds new Cargo feature `override_allocator_on_supported_platforms`.
- `cat` the entire `config.log` (#142)

# 0.6.0 - 2024-07-14

- Fix build on riscv64gc-unknown-linux-musl (#67) (#75)
- Allow jemalloc-sys to be the default allocator on musl linux (#70)
- Add Chimera Linux to gmake targets (#73)
- Add profiling options to jemalloc-ctl (#74)
- Fix jemalloc version not shown in API (#77)
- Fix jemalloc stats is still enabled when stats feature is disabled (#82)
- Fix duplicated symbol when build and link on aarch64-linux-android (#83)
- Revise CI runner platform on macOS (#86)
- Allow setting per-target env (#91)
- Remove outdated clippy allows (#94)
- Set MSRV to 1.71.0 (#95)

Note since 0.6.0, if you want to use jemalloc stats, you have to enable the
feature explicitly.

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
