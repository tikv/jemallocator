// When updating vendored jemalloc, review include/jemalloc/internal/quantum.h
// and update these defaults to match its architecture and ABI rules.
const DEFAULT_LG_QUANTUM: u32 = if cfg!(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm64ec",
    target_arch = "sparc",
    target_arch = "sparc64",
    target_arch = "loongarch32",
    target_arch = "loongarch64",
    target_arch = "mips64",
    target_arch = "mips64r6",
    target_arch = "powerpc",
    target_arch = "powerpc64",
    target_arch = "riscv32",
    target_arch = "riscv64",
    target_arch = "s390x",
)) {
    4
} else if cfg!(any(
    target_arch = "arm",
    target_arch = "m68k",
    target_arch = "mips",
    target_arch = "mips32r6",
)) {
    3
} else {
    0
};

pub(super) const LG_QUANTUM: u32 = match option_env!("JEMALLOC_LG_QUANTUM") {
    Some(value) if !value.is_empty() => parse_lg_quantum(value.as_bytes()),
    _ => DEFAULT_LG_QUANTUM,
};

const fn parse_lg_quantum(value: &[u8]) -> u32 {
    let mut exponent = 0;
    let mut index = 0;
    while index < value.len() {
        let digit = value[index];
        if !digit.is_ascii_digit() {
            return 0;
        }
        exponent = exponent * 10 + (digit - b'0') as u32;
        // quantum.h uses the exponent in a shift of a 32-bit unsigned integer.
        if exponent >= 32 {
            return 0;
        }
        index += 1;
    }
    exponent
}
