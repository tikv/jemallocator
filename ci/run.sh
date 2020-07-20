#!/usr/bin/env sh

set -ex

: "${TARGET?The TARGET environment variable must be set.}"

echo "Running tests for target: ${TARGET}, Rust version=${TRAVIS_RUST_VERSION}"
export RUST_BACKTRACE=1
export RUST_TEST_THREADS=1
export RUST_TEST_NOCAPTURE=1

# FIXME: workaround cargo breaking Travis-CI again:
# https://github.com/rust-lang/cargo/issues/5721
if [ "$TRAVIS" = "true" ]
then
    export TERM=dumb
fi

# Runs jemalloc tests when building jemalloc-sys (runs "make check"):
if [ "${NO_JEMALLOC_TESTS}" = "1" ]
then
    echo "jemalloc's tests are not run"
else
    export JEMALLOC_SYS_RUN_JEMALLOC_TESTS=1
fi

if [ "${VALGRIND}" = "1" ]
then
    case "${TARGET}" in
        "x86_64-unknown-linux-gnu")
            export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER=valgrind
            ;;
        "x86_64-apple-darwin")
            export CARGO_TARGET_X86_64_APPLE_DARWIN_RUNNER=valgrind
            ;;
        *)
            echo "Specify how to run valgrind for TARGET=${TARGET}"
            exit 1
            ;;
    esac
fi

if [ "${TARGET}" = "x86_64-unknown-linux-gnu" ] || [ "${TARGET}" = "x86_64-apple-darwin" ]
then
    # Not using tee to avoid too much logs that exceeds travis' limit.
    if ! cargo build -vv --target "${TARGET}" > build_no_std.txt 2>&1; then
        tail -n 1024 build_no_std.txt
        exit 1
    fi

    # Check that the no-std builds are not linked against a libc with default
    # features or the `use_std` feature enabled:
    ! grep -q "default" build_no_std.txt
    ! grep -q "use_std" build_no_std.txt

    RUST_SYS_ROOT=$(rustc --target="${TARGET}" --print sysroot)
    RUST_LLVM_NM="${RUST_SYS_ROOT}/lib/rustlib/${TARGET}/bin/llvm-nm"

    find target/ -iname '*jemalloc*.rlib' | while read -r rlib; do
        echo "${RUST_LLVM_NM} ${rlib}"
        ! $RUST_LLVM_NM "${rlib}" | grep "std"
    done
fi

cargo test --target "${TARGET}"
cargo test --target "${TARGET}" --features profiling
cargo test --target "${TARGET}" --features debug
cargo test --target "${TARGET}" --features stats
cargo test --target "${TARGET}" --features 'debug profiling'

cargo test --target "${TARGET}" \
    --features unprefixed_malloc_on_supported_platforms
cargo test --target "${TARGET}" --no-default-features
cargo test --target "${TARGET}" --no-default-features \
    --features background_threads_runtime_support

if [ "${NOBGT}" = "1" ]
then
    echo "enabling background threads by default at run-time is not tested"
else
    cargo test --target "${TARGET}" --features background_threads
fi

cargo test --target "${TARGET}" --release
cargo test --target "${TARGET}" --manifest-path jemalloc-sys/Cargo.toml
cargo test --target "${TARGET}" \
             --manifest-path jemalloc-sys/Cargo.toml \
             --features unprefixed_malloc_on_supported_platforms

# FIXME: jemalloc-ctl fails in the following targets
case "${TARGET}" in
    "i686-unknown-linux-musl") ;;
    "x86_64-unknown-linux-musl") ;;
    *)

        cargo test --target "${TARGET}" \
                   --manifest-path jemalloc-ctl/Cargo.toml \
                   --no-default-features
        # FIXME: cross fails to pass features to jemalloc-ctl
        # ${CARGO_CMD} test --target "${TARGET}" \
        #             --manifest-path jemalloc-ctl \
        #             --no-default-features --features use_std
        ;;
esac

cargo test --target "${TARGET}" -p systest
cargo test --target "${TARGET}" --manifest-path jemallocator-global/Cargo.toml
cargo test --target "${TARGET}" \
             --manifest-path jemallocator-global/Cargo.toml \
             --features force_global_jemalloc

# FIXME: Re-enable following test when allocator API is stable again.
# if [ "${TRAVIS_RUST_VERSION}" = "nightly"  ]
# then
#     # The Alloc trait is unstable:
#     ${CARGO_CMD} test --target "${TARGET}" --features alloc_trait
# fi
