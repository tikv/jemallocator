//! Build shared library `dep.c`.
use std::{env, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=src/dep.c");

    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());

    // NOTE: Only for testing, extension is wrong when cross-compiling.
    let dylib = out_dir.join(format!(
        "{}dep{}",
        env::consts::DLL_PREFIX,
        env::consts::DLL_SUFFIX
    ));

    let status = cc::Build::new()
        .get_compiler()
        .to_command()
        .arg("src/dep.c")
        .arg("-shared")
        .arg("-o")
        .arg(&dylib)
        .status()
        .unwrap();
    assert!(status.success());

    println!("cargo:rustc-link-lib=dylib=dep");
    println!("cargo:rustc-link-search=native={}", out_dir.display());
}
