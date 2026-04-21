use std::{
    collections::{BTreeSet, HashMap, HashSet},
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use super::CcBuildContext;

const PRIVATE_NAMESPACE_PREFIX: &str = "_rjem_je_";

const C_SOURCES: &[&str] = &[
    "src/jemalloc.c",
    "src/arena.c",
    "src/background_thread.c",
    "src/base.c",
    "src/bin.c",
    "src/bin_info.c",
    "src/bitmap.c",
    "src/buf_writer.c",
    "src/cache_bin.c",
    "src/ckh.c",
    "src/counter.c",
    "src/ctl.c",
    "src/decay.c",
    "src/div.c",
    "src/ecache.c",
    "src/edata.c",
    "src/edata_cache.c",
    "src/ehooks.c",
    "src/emap.c",
    "src/eset.c",
    "src/exp_grow.c",
    "src/extent.c",
    "src/extent_dss.c",
    "src/extent_mmap.c",
    "src/fxp.c",
    "src/san.c",
    "src/san_bump.c",
    "src/hook.c",
    "src/hpa.c",
    "src/hpa_central.c",
    "src/hpa_hooks.c",
    "src/hpa_utils.c",
    "src/hpdata.c",
    "src/inspect.c",
    "src/large.c",
    "src/log.c",
    "src/malloc_io.c",
    "src/conf.c",
    "src/mutex.c",
    "src/nstime.c",
    "src/pa.c",
    "src/pa_extra.c",
    "src/pac.c",
    "src/pages.c",
    "src/peak_event.c",
    "src/prof.c",
    "src/prof_data.c",
    "src/prof_log.c",
    "src/prof_recent.c",
    "src/prof_stack_range.c",
    "src/prof_stats.c",
    "src/prof_sys.c",
    "src/psset.c",
    "src/rtree.c",
    "src/safety_check.c",
    "src/sc.c",
    "src/sec.c",
    "src/stats.c",
    "src/sz.c",
    "src/tcache.c",
    "src/test_hooks.c",
    "src/thread_event.c",
    "src/thread_event_registry.c",
    "src/ticker.c",
    "src/tsd.c",
    "src/util.c",
    "src/witness.c",
];

const BASE_PUBLIC_SYMBOLS: &[&str] = &[
    "aligned_alloc",
    "calloc",
    "dallocx",
    "free",
    "free_sized",
    "free_aligned_sized",
    "mallctl",
    "mallctlbymib",
    "mallctlnametomib",
    "malloc",
    "malloc_conf",
    "malloc_conf_2_conf_harder",
    "malloc_message",
    "malloc_stats_print",
    "malloc_usable_size",
    "mallocx",
    "nallocx",
    "posix_memalign",
    "rallocx",
    "realloc",
    "sallocx",
    "sdallocx",
    "xallocx",
];

pub(crate) fn build(ctx: &CcBuildContext<'_>) {
    let config = BuildConfig::new(ctx);
    generate_configured_files(ctx, &config);

    if ctx.target.contains("msvc") {
        if ctx.options.profiling {
            panic!("profiling is not yet supported for `cc_build` on MSVC");
        }
    } else {
        generate_private_namespace(ctx, &config);
    }

    compile_archive(ctx, &config);
    install_header(ctx);
}

struct BuildConfig {
    platform: Platform,
    version: JemallocVersion,
    public_symbols: Vec<PublicSymbol>,
    exported_symbols: HashSet<String>,
    source_files: Vec<PathBuf>,
    compiler_flags: Vec<String>,
    compile_defines: Vec<(String, Option<String>)>,
    internal_defines: HashMap<String, Option<String>>,
    public_defines: HashMap<String, Option<String>>,
}

#[derive(Clone)]
struct PublicSymbol {
    base: String,
    actual: String,
}

#[derive(Clone, Copy)]
struct Platform {
    is_windows: bool,
    is_msvc: bool,
    is_apple: bool,
    is_linux: bool,
    is_android: bool,
    is_musl: bool,
    is_64_bit: bool,
    is_x86: bool,
    is_x86_64: bool,
    is_aarch64: bool,
    is_arm: bool,
}

struct JemallocVersion {
    full: String,
    major: String,
    minor: String,
    bugfix: String,
    nrev: String,
    gid: String,
}

impl BuildConfig {
    fn new(ctx: &CcBuildContext<'_>) -> Self {
        let platform = Platform::new(ctx.target);
        let version = JemallocVersion::parse(&ctx.options.je_version);
        let public_symbols = public_symbols(ctx, &platform, &version);
        let exported_symbols = exported_symbols(ctx, &platform, &public_symbols);
        let source_files = source_files(ctx, &platform);
        let compiler_flags = compiler_flags(ctx, &platform);
        let compile_defines = compile_defines(ctx, &platform);
        let internal_defines = internal_defines(ctx, &platform);
        let public_defines = public_defines(ctx, &platform);

        Self {
            platform,
            version,
            public_symbols,
            exported_symbols,
            source_files,
            compiler_flags,
            compile_defines,
            internal_defines,
            public_defines,
        }
    }
}

impl Platform {
    fn new(target: &str) -> Self {
        let arch = target_arch(target);

        let is_windows = target.contains("windows");
        let is_msvc = target.contains("msvc");
        let is_apple =
            target.contains("apple") || target.contains("darwin") || target.contains("ios");
        let is_linux = target.contains("linux");
        let is_android = target.contains("android");
        let is_musl = target.contains("musl");
        let is_x86 =
            arch == "i386" || arch == "i486" || arch == "i586" || arch == "i686" || arch == "x86";
        let is_x86_64 = arch == "x86_64";
        let is_aarch64 = arch == "aarch64";
        let is_arm = arch.starts_with("arm") || arch.starts_with("thumb");
        let is_64_bit = matches!(
            arch,
            "x86_64"
                | "aarch64"
                | "loongarch64"
                | "mips64"
                | "mips64el"
                | "powerpc64"
                | "powerpc64le"
                | "riscv64gc"
                | "riscv64imac"
                | "riscv64a23"
                | "s390x"
                | "sparc64"
        );

        Self {
            is_windows,
            is_msvc,
            is_apple,
            is_linux,
            is_android,
            is_musl,
            is_64_bit,
            is_x86,
            is_x86_64,
            is_aarch64,
            is_arm,
        }
    }
}

impl JemallocVersion {
    fn parse(version: &str) -> Self {
        let (prefix, gid) = version
            .split_once("-g")
            .unwrap_or_else(|| panic!("invalid jemalloc version: {}", version));
        let mut dot_parts = prefix.split('.');
        let major = dot_parts
            .next()
            .unwrap_or_else(|| panic!("missing jemalloc major version: {}", version));
        let minor = dot_parts
            .next()
            .unwrap_or_else(|| panic!("missing jemalloc minor version: {}", version));
        let bugfix_and_nrev = dot_parts
            .next()
            .unwrap_or_else(|| panic!("missing jemalloc bugfix version: {}", version));
        let (bugfix, nrev) = bugfix_and_nrev
            .split_once('-')
            .unwrap_or_else(|| panic!("missing jemalloc revision count: {}", version));

        Self {
            full: version.to_owned(),
            major: major.to_owned(),
            minor: minor.to_owned(),
            bugfix: bugfix.to_owned(),
            nrev: nrev.to_owned(),
            gid: gid.to_owned(),
        }
    }
}

fn generate_configured_files(ctx: &CcBuildContext<'_>, config: &BuildConfig) {
    let include_dir = ctx.build_dir.join("include/jemalloc");
    let internal_dir = include_dir.join("internal");
    fs::create_dir_all(&internal_dir)
        .unwrap_or_else(|e| panic!("failed to create {}: {e}", internal_dir.display()));

    write_public_symbols(
        &internal_dir.join("public_symbols.txt"),
        &config.public_symbols,
    );
    write_public_namespace(
        &internal_dir.join("public_namespace.h"),
        &config.public_symbols,
    );
    write_public_unnamespace(
        &internal_dir.join("public_unnamespace.h"),
        &config.public_symbols,
    );

    render_config_header(
        &ctx.build_dir.join("include/jemalloc/jemalloc_defs.h.in"),
        &include_dir.join("jemalloc_defs.h"),
        &config.public_defines,
        &[],
    );
    render_config_header(
        &ctx.build_dir
            .join("include/jemalloc/internal/jemalloc_internal_defs.h.in"),
        &internal_dir.join("jemalloc_internal_defs.h"),
        &config.internal_defines,
        &[],
    );

    let replacements = vec![
        ("@jemalloc_version@", config.version.full.as_str()),
        ("@jemalloc_version_major@", config.version.major.as_str()),
        ("@jemalloc_version_minor@", config.version.minor.as_str()),
        ("@jemalloc_version_bugfix@", config.version.bugfix.as_str()),
        ("@jemalloc_version_nrev@", config.version.nrev.as_str()),
        ("@jemalloc_version_gid@", config.version.gid.as_str()),
    ];
    render_plain_template(
        &ctx.build_dir.join("include/jemalloc/jemalloc_macros.h.in"),
        &include_dir.join("jemalloc_macros.h"),
        &replacements,
    );
    render_plain_template(
        &ctx.build_dir.join("include/jemalloc/jemalloc_protos.h.in"),
        &include_dir.join("jemalloc_protos.h"),
        &[("@je_@", "je_"), ("@install_suffix@", "")],
    );
    render_plain_template(
        &ctx.build_dir.join("include/jemalloc/jemalloc_protos.h.in"),
        &include_dir.join("jemalloc_protos_jet.h"),
        &[("@je_@", "jet_"), ("@install_suffix@", "")],
    );
    render_plain_template(
        &ctx.build_dir
            .join("include/jemalloc/jemalloc_typedefs.h.in"),
        &include_dir.join("jemalloc_typedefs.h"),
        &[],
    );
    render_plain_template(
        &ctx.build_dir
            .join("include/jemalloc/internal/jemalloc_preamble.h.in"),
        &internal_dir.join("jemalloc_preamble.h"),
        &[
            ("@install_suffix@", ""),
            ("@private_namespace@", PRIVATE_NAMESPACE_PREFIX),
        ],
    );

    write_rename_header(
        &include_dir.join("jemalloc_rename.h"),
        &config.public_symbols,
    );
    write_mangle_header(
        &include_dir.join("jemalloc_mangle.h"),
        &config.public_symbols,
        "je_",
    );
    write_mangle_header(
        &include_dir.join("jemalloc_mangle_jet.h"),
        &config.public_symbols,
        "jet_",
    );
    write_jemalloc_header(
        &include_dir.join("jemalloc.h"),
        &[
            include_dir.join("jemalloc_defs.h"),
            include_dir.join("jemalloc_rename.h"),
            include_dir.join("jemalloc_macros.h"),
            include_dir.join("jemalloc_protos.h"),
            include_dir.join("jemalloc_typedefs.h"),
            include_dir.join("jemalloc_mangle.h"),
        ],
    );
}

fn compile_archive(ctx: &CcBuildContext<'_>, config: &BuildConfig) {
    let lib_dir = ctx.out_dir.join("lib");
    fs::create_dir_all(&lib_dir)
        .unwrap_or_else(|e| panic!("failed to create {}: {e}", lib_dir.display()));

    compile_library(ctx, config, &lib_dir, "jemalloc", false);
    if !config.platform.is_windows {
        compile_library(ctx, config, &lib_dir, "jemalloc_pic", true);
    }
}

fn compile_library(
    ctx: &CcBuildContext<'_>,
    config: &BuildConfig,
    out_dir: &Path,
    lib_name: &str,
    pic: bool,
) {
    let mut build = base_cc_build(ctx, config, out_dir);

    if pic && !config.platform.is_windows {
        build.flag("-fPIC");
        build.flag("-DPIC");
    }

    if config.platform.is_msvc {
        build.define("JEMALLOC_NO_PRIVATE_NAMESPACE", None::<&str>);
        build.define("JEMALLOC_EXPORT", Some(""));
        build.define("JEMALLOC_STATIC", None::<&str>);
        build.define("_LIB", None::<&str>);
    }

    for source in &config.source_files {
        build.file(source);
    }

    build.compile(lib_name);
}

fn generate_private_namespace(ctx: &CcBuildContext<'_>, config: &BuildConfig) {
    let private_namespace_dir = ctx.out_dir.join("cc_build/private_namespace");
    if private_namespace_dir.exists() {
        fs::remove_dir_all(&private_namespace_dir).unwrap_or_else(|e| {
            panic!("failed to remove {}: {e}", private_namespace_dir.display())
        });
    }

    let mut build = base_cc_build(ctx, config, &private_namespace_dir);
    build.define("JEMALLOC_NO_PRIVATE_NAMESPACE", None::<&str>);
    if config.platform.is_msvc {
        build.define("JEMALLOC_EXPORT", Some(""));
        build.define("JEMALLOC_STATIC", None::<&str>);
        build.define("_LIB", None::<&str>);
    }

    for source in &config.source_files {
        build.file(source);
    }

    let objects = build.compile_intermediates();
    let mut symbols = BTreeSet::new();
    let nm_sym_prefix = if config.platform.is_apple { "_" } else { "" };

    for object in &objects {
        let output = dump_symbols(object);
        for symbol in parse_nm_symbols(&output, &config.exported_symbols, nm_sym_prefix) {
            if config.platform.is_apple && symbol == "zone_register" {
                continue;
            }
            symbols.insert(symbol);
        }
    }

    if config.platform.is_apple {
        let zone_symbol = if ctx.options.use_prefix {
            "_rjem_je_zone_register"
        } else {
            "je_zone_register"
        };
        symbols.insert(format!("__zone_register_alias__={zone_symbol}"));
    }

    let private_namespace_path = ctx
        .build_dir
        .join("include/jemalloc/internal/private_namespace.h");
    let mut rendered = String::new();

    for symbol in symbols {
        if let Some(alias) = symbol.strip_prefix("__zone_register_alias__=") {
            rendered.push_str("#define zone_register ");
            rendered.push_str(alias);
            rendered.push('\n');
            continue;
        }

        rendered.push_str("#define ");
        rendered.push_str(&symbol);
        rendered.push_str(" JEMALLOC_N(");
        rendered.push_str(&symbol);
        rendered.push_str(")\n");
    }

    fs::write(&private_namespace_path, rendered)
        .unwrap_or_else(|e| panic!("failed to write {}: {e}", private_namespace_path.display()));
}

fn install_header(ctx: &CcBuildContext<'_>) {
    let include_dir = ctx.out_dir.join("include/jemalloc");
    fs::create_dir_all(&include_dir)
        .unwrap_or_else(|e| panic!("failed to create {}: {e}", include_dir.display()));

    let src = ctx.build_dir.join("include/jemalloc/jemalloc.h");
    let dst = include_dir.join("jemalloc.h");
    fs::copy(&src, &dst)
        .unwrap_or_else(|e| panic!("failed to copy {} to {}: {e}", src.display(), dst.display()));
}

fn base_cc_build(ctx: &CcBuildContext<'_>, config: &BuildConfig, out_dir: &Path) -> cc::Build {
    let mut build = cc::Build::new();
    build
        .target(ctx.target)
        .host(ctx.host)
        .out_dir(out_dir)
        .cargo_metadata(false)
        .no_default_flags(true)
        .warnings(false)
        .extra_warnings(false)
        .pic(false)
        .include(ctx.build_dir.join("include"))
        .include(ctx.build_dir.join("include/jemalloc"))
        .include(ctx.build_dir.join("include/jemalloc/internal"))
        .include(ctx.build_dir.join("src"));

    if config.platform.is_msvc {
        build.include(ctx.build_dir.join("include/msvc_compat"));
    }

    for flag in &config.compiler_flags {
        build.flag(flag);
    }

    for (name, value) in &config.compile_defines {
        match value {
            Some(value) => build.define(name, value.as_str()),
            None => build.define(name, None::<&str>),
        };
    }

    build
}

fn public_symbols(
    ctx: &CcBuildContext<'_>,
    platform: &Platform,
    version: &JemallocVersion,
) -> Vec<PublicSymbol> {
    let mut bases = BASE_PUBLIC_SYMBOLS
        .iter()
        .map(|symbol| (*symbol).to_owned())
        .collect::<Vec<_>>();
    bases.insert(16, format!("smallocx_{}", version.gid));

    if has_memalign(platform) {
        bases.push("memalign".to_owned());
    }
    if has_valloc(platform) {
        bases.push("valloc".to_owned());
    }
    if has_pvalloc(platform) {
        bases.push("pvalloc".to_owned());
    }
    if platform.is_apple {
        bases.push("malloc_size".to_owned());
    }

    let prefix = if ctx.options.use_prefix { "_rjem_" } else { "" };
    bases
        .into_iter()
        .map(|base| PublicSymbol {
            actual: format!("{prefix}{base}"),
            base,
        })
        .collect()
}

fn exported_symbols(
    ctx: &CcBuildContext<'_>,
    platform: &Platform,
    public_symbols: &[PublicSymbol],
) -> HashSet<String> {
    let mut exported = public_symbols
        .iter()
        .map(|symbol| symbol.actual.clone())
        .collect::<HashSet<_>>();

    if !platform.is_windows {
        exported.insert("pthread_create".to_owned());
    }

    if platform.is_windows {
        exported.insert("tls_callback".to_owned());
    }

    if !ctx.options.use_prefix && platform.is_linux && !platform.is_musl {
        exported.extend(
            [
                "__libc_calloc",
                "__libc_free",
                "__libc_free_sized",
                "__libc_free_aligned_sized",
                "__libc_malloc",
                "__libc_memalign",
                "__libc_realloc",
                "__libc_valloc",
                "__libc_pvalloc",
                "__posix_memalign",
                "__free_hook",
                "__malloc_hook",
                "__realloc_hook",
                "__memalign_hook",
            ]
            .iter()
            .map(|symbol| (*symbol).to_owned()),
        );
    }

    exported
}

fn source_files(ctx: &CcBuildContext<'_>, platform: &Platform) -> Vec<PathBuf> {
    let mut sources = C_SOURCES
        .iter()
        .map(|source| ctx.build_dir.join(source))
        .collect::<Vec<_>>();
    if platform.is_apple {
        sources.push(ctx.build_dir.join("src/zone.c"));
    }
    sources
}

fn compiler_flags(ctx: &CcBuildContext<'_>, platform: &Platform) -> Vec<String> {
    let mut flags = Vec::new();

    if platform.is_linux || platform.is_android {
        flags.push("-D_GNU_SOURCE".to_owned());
    }
    if !platform.is_windows {
        flags.push("-D_REENTRANT".to_owned());
    }

    if !ctx.options.debug {
        if platform.is_msvc {
            flags.push("-O2".to_owned());
            flags.push("/nologo".to_owned());
        } else {
            flags.push("-O3".to_owned());
            flags.push("-funroll-loops".to_owned());
        }
    }

    if !platform.is_msvc && !platform.is_apple {
        flags.push("-fvisibility=hidden".to_owned());
    }

    if ctx.options.profiling && !platform.is_msvc {
        flags.push("-fno-omit-frame-pointer".to_owned());
    }

    flags
}

fn compile_defines(ctx: &CcBuildContext<'_>, platform: &Platform) -> Vec<(String, Option<String>)> {
    let mut defines = Vec::new();

    if platform.is_msvc {
        defines.push(("JEMALLOC_EXPORT".to_owned(), Some(String::new())));
        defines.push(("JEMALLOC_STATIC".to_owned(), None));
    }

    if !ctx.options.debug {
        defines.push(("NDEBUG".to_owned(), None));
    }

    defines
}

fn public_defines(
    _ctx: &CcBuildContext<'_>,
    platform: &Platform,
) -> HashMap<String, Option<String>> {
    let mut defines = HashMap::new();

    if !platform.is_msvc {
        define_empty(&mut defines, "JEMALLOC_HAVE_ATTR");
        define_empty(&mut defines, "JEMALLOC_HAVE_ATTR_ALLOC_SIZE");
        define_empty(&mut defines, "JEMALLOC_HAVE_ATTR_FORMAT_ARG");
        define_empty(&mut defines, "JEMALLOC_HAVE_ATTR_FORMAT_GNU_PRINTF");
        define_empty(&mut defines, "JEMALLOC_HAVE_ATTR_FORMAT_PRINTF");
        define_empty(&mut defines, "JEMALLOC_HAVE_ATTR_FALLTHROUGH");
        define_empty(&mut defines, "JEMALLOC_HAVE_ATTR_COLD");
        define_empty(&mut defines, "JEMALLOC_HAVE_ATTR_DEPRECATED");
    }

    if has_memalign(platform) {
        define_empty(&mut defines, "JEMALLOC_OVERRIDE_MEMALIGN");
    }
    if has_valloc(platform) {
        define_empty(&mut defines, "JEMALLOC_OVERRIDE_VALLOC");
    }
    if has_pvalloc(platform) {
        define_empty(&mut defines, "JEMALLOC_OVERRIDE_PVALLOC");
    }

    define_value(
        &mut defines,
        "JEMALLOC_USABLE_SIZE_CONST",
        if platform.is_linux || platform.is_android {
            ""
        } else {
            "const"
        },
    );

    if platform.is_linux && !platform.is_musl {
        define_empty(&mut defines, "JEMALLOC_USE_CXX_THROW");
    }
    define_empty(&mut defines, "JEMALLOC_CONFIG_ENV");
    define_empty(&mut defines, "JEMALLOC_CONFIG_FILE");

    if platform.is_msvc {
        define_value(&mut defines, "LG_SIZEOF_PTR", "LG_SIZEOF_PTR_WIN");
    } else {
        define_value(
            &mut defines,
            "LG_SIZEOF_PTR",
            if platform.is_64_bit { "3" } else { "2" },
        );
    }

    defines
}

fn internal_defines(
    ctx: &CcBuildContext<'_>,
    platform: &Platform,
) -> HashMap<String, Option<String>> {
    let mut defines = HashMap::new();

    if platform.is_msvc {
        define_empty(&mut defines, "JEMALLOC_EXPORT");
    }

    if ctx.options.use_prefix {
        define_value(&mut defines, "JEMALLOC_PREFIX", "\"_rjem_\"");
        define_value(&mut defines, "JEMALLOC_CPREFIX", "\"_RJEM_\"");
    } else {
        define_empty(&mut defines, "JEMALLOC_IS_MALLOC");
    }

    if !ctx.options.use_prefix && platform.is_linux && !platform.is_musl {
        for name in [
            "JEMALLOC_OVERRIDE___LIBC_CALLOC",
            "JEMALLOC_OVERRIDE___LIBC_FREE",
            "JEMALLOC_OVERRIDE___LIBC_FREE_SIZED",
            "JEMALLOC_OVERRIDE___LIBC_FREE_ALIGNED_SIZED",
            "JEMALLOC_OVERRIDE___LIBC_MALLOC",
            "JEMALLOC_OVERRIDE___LIBC_MEMALIGN",
            "JEMALLOC_OVERRIDE___LIBC_REALLOC",
            "JEMALLOC_OVERRIDE___LIBC_VALLOC",
            "JEMALLOC_OVERRIDE___LIBC_PVALLOC",
            "JEMALLOC_OVERRIDE___POSIX_MEMALIGN",
            "JEMALLOC_GLIBC_MALLOC_HOOK",
            "JEMALLOC_GLIBC_MEMALIGN_HOOK",
        ] {
            define_empty(&mut defines, name);
        }
    }

    define_value(
        &mut defines,
        "JEMALLOC_PRIVATE_NAMESPACE",
        PRIVATE_NAMESPACE_PREFIX,
    );

    define_value(
        &mut defines,
        "CPU_SPINWAIT",
        if platform.is_msvc {
            ""
        } else if platform.is_x86 || platform.is_x86_64 {
            "__asm__ volatile(\"pause\")"
        } else if platform.is_arm || platform.is_aarch64 {
            "__asm__ volatile(\"isb\")"
        } else {
            ""
        },
    );
    define_value(
        &mut defines,
        "HAVE_CPU_SPINWAIT",
        if platform.is_msvc {
            "0"
        } else if platform.is_x86 || platform.is_x86_64 || platform.is_arm || platform.is_aarch64 {
            "1"
        } else {
            "0"
        },
    );

    if let Some(ref lg_vaddr) = ctx.options.lg_vaddr {
        define_value(&mut defines, "LG_VADDR", lg_vaddr);
    } else if platform.is_aarch64 {
        define_value(
            &mut defines,
            "LG_VADDR",
            if platform.is_64_bit { "48" } else { "32" },
        );
    } else if platform.is_x86_64 {
        define_value(
            &mut defines,
            "LG_VADDR",
            if platform.is_linux { "48" } else { "57" },
        );
    } else if platform.is_64_bit {
        define_value(&mut defines, "LG_VADDR", "64");
    } else {
        define_value(&mut defines, "LG_VADDR", "32");
    }

    if !platform.is_msvc {
        define_empty(&mut defines, "JEMALLOC_C11_ATOMICS");
        define_empty(&mut defines, "JEMALLOC_GCC_ATOMIC_ATOMICS");
        define_empty(&mut defines, "JEMALLOC_GCC_U8_ATOMIC_ATOMICS");
        define_empty(&mut defines, "JEMALLOC_GCC_SYNC_ATOMICS");
        define_empty(&mut defines, "JEMALLOC_GCC_U8_SYNC_ATOMICS");
        define_empty(&mut defines, "JEMALLOC_HAVE_BUILTIN_CLZ");
    }

    if platform.is_linux {
        define_empty(&mut defines, "JEMALLOC_USE_SYSCALL");
    }
    if platform.is_linux && !platform.is_musl {
        define_empty(&mut defines, "JEMALLOC_HAVE_SECURE_GETENV");
    }
    if !platform.is_windows {
        define_empty(&mut defines, "JEMALLOC_HAVE_PTHREAD_ATFORK");
    }
    if platform.is_linux {
        define_empty(&mut defines, "JEMALLOC_HAVE_PTHREAD_SETNAME_NP");
        define_empty(&mut defines, "JEMALLOC_HAVE_PTHREAD_GETNAME_NP");
        define_empty(&mut defines, "JEMALLOC_HAVE_CLOCK_MONOTONIC_COARSE");
        define_empty(&mut defines, "JEMALLOC_HAVE_CLOCK_MONOTONIC");
        define_empty(&mut defines, "JEMALLOC_HAVE_CLOCK_REALTIME");
        define_empty(&mut defines, "JEMALLOC_HAVE_PRCTL");
    }
    if platform.is_apple {
        define_empty(&mut defines, "JEMALLOC_HAVE_MACH_ABSOLUTE_TIME");
        define_empty(&mut defines, "JEMALLOC_HAVE_CLOCK_GETTIME_NSEC_NP");
        define_empty(&mut defines, "JEMALLOC_HAVE_MALLOC_SIZE");
    }

    if platform.is_linux || platform.is_android || platform.is_musl {
        define_empty(&mut defines, "JEMALLOC_THREADED_INIT");
    }

    if !ctx.options.disable_initial_exec_tls
        && !platform.is_windows
        && !platform.is_apple
        && !platform.is_android
    {
        define_value(
            &mut defines,
            "JEMALLOC_TLS_MODEL",
            "__attribute__((tls_model(\"initial-exec\")))",
        );
    } else {
        define_empty(&mut defines, "JEMALLOC_TLS_MODEL");
    }

    if ctx.options.debug {
        define_empty(&mut defines, "JEMALLOC_DEBUG");
    }
    if ctx.options.stats {
        define_empty(&mut defines, "JEMALLOC_STATS");
    }
    if ctx.options.profiling {
        define_empty(&mut defines, "JEMALLOC_PROF");
        if platform.is_linux && !platform.is_msvc {
            define_empty(&mut defines, "JEMALLOC_PROF_GCC");
        }
    }

    if !platform.is_windows {
        define_empty(&mut defines, "JEMALLOC_DSS");
    }
    define_empty(&mut defines, "JEMALLOC_FILL");

    if let Some(ref lg_quantum) = ctx.options.lg_quantum {
        define_value(&mut defines, "LG_QUANTUM", lg_quantum);
    }
    if let Some(ref lg_page) = ctx.options.lg_page {
        define_value(&mut defines, "LG_PAGE", lg_page);
    } else {
        define_value(&mut defines, "LG_PAGE", &default_lg_page(ctx, platform));
    }

    if let Some(ref lg_hugepage) = ctx.options.lg_hugepage {
        define_value(&mut defines, "LG_HUGEPAGE", lg_hugepage);
    } else {
        define_value(&mut defines, "LG_HUGEPAGE", &default_lg_hugepage(ctx));
    }

    if !platform.is_windows {
        define_empty(&mut defines, "JEMALLOC_MAPS_COALESCE");
    }
    if platform.is_64_bit {
        define_empty(&mut defines, "JEMALLOC_RETAIN");
    }
    if !platform.is_windows && !platform.is_apple && !platform.is_android {
        define_empty(&mut defines, "JEMALLOC_TLS");
    }

    if platform.is_msvc {
        define_value(&mut defines, "JEMALLOC_INTERNAL_UNREACHABLE", "abort");
        define_value(&mut defines, "JEMALLOC_INTERNAL_FFSLL", "ffsll");
        define_value(&mut defines, "JEMALLOC_INTERNAL_FFSL", "ffsl");
        define_value(&mut defines, "JEMALLOC_INTERNAL_FFS", "ffs");
    } else {
        define_value(
            &mut defines,
            "JEMALLOC_INTERNAL_UNREACHABLE",
            "__builtin_unreachable",
        );
        define_value(&mut defines, "JEMALLOC_INTERNAL_FFSLL", "__builtin_ffsll");
        define_value(&mut defines, "JEMALLOC_INTERNAL_FFSL", "__builtin_ffsl");
        define_value(&mut defines, "JEMALLOC_INTERNAL_FFS", "__builtin_ffs");
        define_value(
            &mut defines,
            "JEMALLOC_INTERNAL_POPCOUNTL",
            "__builtin_popcountl",
        );
        define_value(
            &mut defines,
            "JEMALLOC_INTERNAL_POPCOUNT",
            "__builtin_popcount",
        );
    }

    if !ctx.options.disable_cache_oblivious {
        define_empty(&mut defines, "JEMALLOC_CACHE_OBLIVIOUS");
    }

    if platform.is_apple {
        define_empty(&mut defines, "JEMALLOC_ZONE");
    }
    if platform.is_linux || platform.is_android {
        define_empty(&mut defines, "JEMALLOC_PROC_SYS_VM_OVERCOMMIT_MEMORY");
    }
    if platform.is_linux || platform.is_android || platform.is_apple {
        define_empty(&mut defines, "JEMALLOC_HAVE_MADVISE");
    }
    if platform.is_linux {
        define_empty(&mut defines, "JEMALLOC_HAVE_MADVISE_HUGE");
        define_empty(&mut defines, "JEMALLOC_HAVE_MADVISE_COLLAPSE");
        define_empty(&mut defines, "JEMALLOC_PURGE_MADVISE_DONTNEED");
        define_empty(&mut defines, "JEMALLOC_PURGE_MADVISE_DONTNEED_ZEROS");
        define_empty(&mut defines, "JEMALLOC_MADVISE_DONTDUMP");
    }
    if platform.is_linux && !platform.is_msvc {
        define_empty(&mut defines, "JEMALLOC_HAVE_MPROTECT");
    }
    if platform.is_linux {
        define_empty(&mut defines, "JEMALLOC_HAS_ALLOCA_H");
    }
    if !platform.is_msvc {
        define_empty(&mut defines, "JEMALLOC_HAS_RESTRICT");
    }

    define_value(
        &mut defines,
        "LG_SIZEOF_INT",
        if platform.is_64_bit || platform.is_windows {
            "2"
        } else {
            "2"
        },
    );
    define_value(
        &mut defines,
        "LG_SIZEOF_LONG",
        if platform.is_windows {
            "2"
        } else if platform.is_64_bit {
            "3"
        } else {
            "2"
        },
    );
    define_value(&mut defines, "LG_SIZEOF_LONG_LONG", "3");
    define_value(&mut defines, "LG_SIZEOF_INTMAX_T", "3");

    if !platform.is_windows {
        define_empty(&mut defines, "JEMALLOC_HAVE_PTHREAD");
    }
    if !platform.is_windows {
        define_empty(&mut defines, "JEMALLOC_HAVE_DLSYM");
    }
    if platform.is_linux {
        define_empty(&mut defines, "JEMALLOC_HAVE_PTHREAD_MUTEX_ADAPTIVE_NP");
        define_empty(&mut defines, "JEMALLOC_HAVE_GETTID");
        define_empty(&mut defines, "JEMALLOC_HAVE_SCHED_GETCPU");
        define_empty(&mut defines, "JEMALLOC_HAVE_SCHED_SETAFFINITY");
        define_empty(&mut defines, "JEMALLOC_HAVE_PTHREAD_SETAFFINITY_NP");
    }

    if !platform.is_windows && !platform.is_apple {
        define_empty(&mut defines, "JEMALLOC_BACKGROUND_THREAD");
    }

    define_value(
        &mut defines,
        "JEMALLOC_CONFIG_MALLOC_CONF",
        &c_string_literal(&ctx.options.malloc_conf),
    );

    if platform.is_linux && !platform.is_musl {
        define_empty(
            &mut defines,
            "JEMALLOC_STRERROR_R_RETURNS_CHAR_WITH_GNU_SOURCE",
        );
    }
    if platform.is_linux || platform.is_android || platform.is_windows {
        define_empty(&mut defines, "JEMALLOC_ZERO_REALLOC_DEFAULT_FREE");
    }
    if !platform.is_msvc {
        define_empty(&mut defines, "JEMALLOC_HAVE_ASM_VOLATILE");
        define_empty(&mut defines, "JEMALLOC_HAVE_INT128");
    }

    defines
}

fn has_memalign(platform: &Platform) -> bool {
    !platform.is_windows
}

fn has_valloc(platform: &Platform) -> bool {
    !platform.is_windows
}

fn has_pvalloc(platform: &Platform) -> bool {
    !platform.is_windows
}

fn default_lg_page(ctx: &CcBuildContext<'_>, platform: &Platform) -> String {
    if ctx.target.contains("aarch64-apple-darwin") {
        return "14".to_owned();
    }
    if ctx.target.contains("aarch64-unknown-linux-") {
        return "16".to_owned();
    }
    if platform.is_apple && platform.is_aarch64 {
        return "14".to_owned();
    }
    if platform.is_windows {
        return "12".to_owned();
    }
    if platform.is_linux && platform.is_64_bit && platform.is_aarch64 {
        return "16".to_owned();
    }
    "12".to_owned()
}

fn default_lg_hugepage(ctx: &CcBuildContext<'_>) -> String {
    if ctx.host == ctx.target {
        if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
            for line in meminfo.lines() {
                let Some(rest) = line.strip_prefix("Hugepagesize:") else {
                    continue;
                };
                let value = rest.split_whitespace().next().unwrap_or_default();
                if let Ok(mut kb) = value.parse::<u64>() {
                    let mut lg = 10;
                    while kb > 1 {
                        kb /= 2;
                        lg += 1;
                    }
                    return lg.to_string();
                }
            }
        }
    }
    "21".to_owned()
}

fn render_config_header(
    template: &Path,
    output: &Path,
    defines: &HashMap<String, Option<String>>,
    replacements: &[(&str, &str)],
) {
    let mut contents = fs::read_to_string(template)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", template.display()));
    for (from, to) in replacements {
        contents = contents.replace(from, to);
    }

    let mut rendered = String::new();
    for line in contents.lines() {
        let trimmed = line.trim();
        if let Some(name) = trimmed.strip_prefix("#undef ") {
            if let Some(value) = defines.get(name) {
                rendered.push_str("#define ");
                rendered.push_str(name);
                if let Some(value) = value {
                    if !value.is_empty() {
                        rendered.push(' ');
                        rendered.push_str(value);
                    } else {
                        rendered.push(' ');
                    }
                }
                rendered.push('\n');
                continue;
            }
        }
        rendered.push_str(line);
        rendered.push('\n');
    }

    fs::write(output, rendered)
        .unwrap_or_else(|e| panic!("failed to write {}: {e}", output.display()));
}

fn render_plain_template(template: &Path, output: &Path, replacements: &[(&str, &str)]) {
    let mut contents = fs::read_to_string(template)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", template.display()));
    for (from, to) in replacements {
        contents = contents.replace(from, to);
    }
    fs::write(output, contents)
        .unwrap_or_else(|e| panic!("failed to write {}: {e}", output.display()));
}

fn write_public_symbols(path: &Path, public_symbols: &[PublicSymbol]) {
    let mut contents = String::new();
    for symbol in public_symbols {
        contents.push_str(&symbol.base);
        contents.push(':');
        contents.push_str(&symbol.actual);
        contents.push('\n');
    }
    fs::write(path, contents).unwrap_or_else(|e| panic!("failed to write {}: {e}", path.display()));
}

fn write_public_namespace(path: &Path, public_symbols: &[PublicSymbol]) {
    let mut contents = String::new();
    for symbol in public_symbols {
        contents.push_str("#define je_");
        contents.push_str(&symbol.base);
        contents.push_str(" JEMALLOC_N(");
        contents.push_str(&symbol.base);
        contents.push_str(")\n");
    }
    fs::write(path, contents).unwrap_or_else(|e| panic!("failed to write {}: {e}", path.display()));
}

fn write_public_unnamespace(path: &Path, public_symbols: &[PublicSymbol]) {
    let mut contents = String::new();
    for symbol in public_symbols {
        contents.push_str("#undef je_");
        contents.push_str(&symbol.base);
        contents.push('\n');
    }
    fs::write(path, contents).unwrap_or_else(|e| panic!("failed to write {}: {e}", path.display()));
}

fn write_rename_header(path: &Path, public_symbols: &[PublicSymbol]) {
    let mut contents = String::new();
    contents.push_str("/*\n");
    contents.push_str(" * Name mangling for public symbols is controlled by --with-mangling and\n");
    contents.push_str(
        " * --with-jemalloc-prefix.  With default settings the je_ prefix is stripped by\n",
    );
    contents.push_str(" * these macro definitions.\n");
    contents.push_str(" */\n");
    contents.push_str("#ifndef JEMALLOC_NO_RENAME\n");
    for symbol in public_symbols {
        contents.push_str("#  define je_");
        contents.push_str(&symbol.base);
        contents.push(' ');
        contents.push_str(&symbol.actual);
        contents.push('\n');
    }
    contents.push_str("#endif\n");
    fs::write(path, contents).unwrap_or_else(|e| panic!("failed to write {}: {e}", path.display()));
}

fn write_mangle_header(path: &Path, public_symbols: &[PublicSymbol], symbol_prefix: &str) {
    let mut contents = String::new();
    contents.push_str("/*\n");
    contents.push_str(
        " * By default application code must explicitly refer to mangled symbol names,\n",
    );
    contents.push_str(
        " * so that it is possible to use jemalloc in conjunction with another allocator\n",
    );
    contents.push_str(
        " * in the same application.  Define JEMALLOC_MANGLE in order to cause automatic\n",
    );
    contents
        .push_str(" * name mangling that matches the API prefixing that happened as a result of\n");
    contents.push_str(" * --with-mangling and/or --with-jemalloc-prefix configuration settings.\n");
    contents.push_str(" */\n");
    contents.push_str("#ifdef JEMALLOC_MANGLE\n");
    contents.push_str("#  ifndef JEMALLOC_NO_DEMANGLE\n");
    contents.push_str("#    define JEMALLOC_NO_DEMANGLE\n");
    contents.push_str("#  endif\n");
    for symbol in public_symbols {
        contents.push_str("#  define ");
        contents.push_str(&symbol.base);
        contents.push(' ');
        contents.push_str(symbol_prefix);
        contents.push_str(&symbol.base);
        contents.push('\n');
    }
    contents.push_str("#endif\n\n");
    contents.push_str("/*\n");
    contents.push_str(
        " * The stable prefixed macros can be used as alternative names for the public\n",
    );
    contents.push_str(" * jemalloc API if JEMALLOC_NO_DEMANGLE is defined.\n");
    contents.push_str(" */\n");
    contents.push_str("#ifndef JEMALLOC_NO_DEMANGLE\n");
    for symbol in public_symbols {
        contents.push_str("#  undef ");
        contents.push_str(symbol_prefix);
        contents.push_str(&symbol.base);
        contents.push('\n');
    }
    contents.push_str("#endif\n");
    fs::write(path, contents).unwrap_or_else(|e| panic!("failed to write {}: {e}", path.display()));
}

fn write_jemalloc_header(path: &Path, parts: &[PathBuf]) {
    let mut contents = String::new();
    contents.push_str("#ifndef JEMALLOC_H_\n");
    contents.push_str("#define JEMALLOC_H_\n");
    contents.push_str("#pragma GCC system_header\n\n");

    for part in parts {
        let text = fs::read_to_string(part)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", part.display()));
        contents.push_str(&text);
        if !text.ends_with('\n') {
            contents.push('\n');
        }
        contents.push('\n');
    }

    contents.push_str("#endif /* JEMALLOC_H_ */\n");
    fs::write(path, contents).unwrap_or_else(|e| panic!("failed to write {}: {e}", path.display()));
}

fn dump_symbols(object: &Path) -> String {
    let mut cmd = Command::new("nm");
    cmd.arg("-a").arg(object);

    println!("running: {cmd:?}");

    let output = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to execute symbol dump command {:?}: {e}", cmd));
    if !output.status.success() {
        panic!(
            "symbol dump command did not execute successfully: {:?}\nexpected success, got: {}",
            cmd, output.status
        );
    }

    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn parse_nm_symbols(
    output: &str,
    exported_symbols: &HashSet<String>,
    nm_sym_prefix: &str,
) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| {
            let parts = line.split_whitespace().collect::<Vec<_>>();
            if parts.len() != 3 {
                return None;
            }

            let symbol_type = parts[1];
            let raw_symbol = parts[2];

            if symbol_type.len() != 1
                || !matches!(
                    symbol_type.as_bytes()[0],
                    b'A' | b'B' | b'C' | b'D' | b'G' | b'R' | b'S' | b'T' | b'V' | b'W'
                )
                || !raw_symbol
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
                || exported_symbols.contains(raw_symbol)
            {
                return None;
            }

            Some(
                raw_symbol
                    .strip_prefix(nm_sym_prefix)
                    .unwrap_or(raw_symbol)
                    .to_owned(),
            )
        })
        .collect()
}

fn define_empty(defines: &mut HashMap<String, Option<String>>, name: &str) {
    defines.insert(name.to_owned(), Some(String::new()));
}

fn define_value(
    defines: &mut HashMap<String, Option<String>>,
    name: &str,
    value: impl Into<String>,
) {
    defines.insert(name.to_owned(), Some(value.into()));
}

fn c_string_literal(value: &str) -> String {
    let mut escaped = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(ch),
        }
    }
    escaped.push('"');
    escaped
}

fn target_arch(target: &str) -> &str {
    target.split('-').next().unwrap_or(target)
}

fn _os_str_to_string(value: &OsStr) -> String {
    value.to_string_lossy().into_owned()
}
