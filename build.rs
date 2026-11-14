use std::env;
use std::path::{Path, PathBuf};

// built outside the repo, see deps/info.txt
fn dep_dir(var: &str) -> Option<PathBuf> {
    println!("cargo:rerun-if-env-changed={var}");
    env::var_os(var).map(PathBuf::from)
}

// headers only trigger rebuilds
fn sources(dir: &str, extension: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();

    for entry in Path::new(dir).read_dir().unwrap() {
        let path = entry.unwrap().path();
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("h") => println!("cargo:rerun-if-changed={}", path.display()),
            Some(ext) if ext == extension => {
                println!("cargo:rerun-if-changed={}", path.display());
                found.push(path);
            }
            _ => {}
        }
    }

    found.sort();
    found
}

// single-config generators build into the dir itself, multi-config (Visual Studio) into Release
fn link_search(dir: PathBuf) {
    for candidate in [dir.clone(), dir.join("Release")] {
        if candidate.is_dir() {
            println!("cargo:rustc-link-search=native={}", candidate.display());
        }
    }
}

fn main() {
    println!("cargo::rustc-check-cfg=cfg(mhttp_engine)");

    // without deps only the Rust side compiles, enough for clippy
    let (Some(boringssl), Some(nghttp2), Some(libuv), Some(cppcoro)) = (
        dep_dir("BORINGSSL_DIR"),
        dep_dir("NGHTTP2_DIR"),
        dep_dir("LIBUV_DIR"),
        dep_dir("CPPCORO_DIR"),
    ) else {
        println!(
            "cargo:warning=BORINGSSL_DIR, NGHTTP2_DIR, LIBUV_DIR or CPPCORO_DIR not set; \
             compiling the Rust side alone (see deps/info.txt)"
        );
        return;
    };

    let msvc = env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    let release = env::var("PROFILE").as_deref() == Ok("release");
    let logging = env::var_os("CARGO_FEATURE_DEBUG_LOG").is_some();
    let deps = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("deps");

    let includes = [
        PathBuf::from("native"),
        deps.join("cxx-async/cxx-async/include"),
        cppcoro.join("include"),
        boringssl.join("include"),
        nghttp2.join("lib/includes"),
        nghttp2.join("build/lib/includes"),
        libuv.join("include"),
    ];

    // uvtls is C
    let mut c = cc::Build::new();
    c.files(sources("native/uvtls", "c"))
        .includes(&includes)
        .warnings(false);
    if release {
        c.define("NDEBUG", None);
    }
    c.compile("mhttp_uvtls");

    let mut cpp = cxx_build::bridge("src/bridge.rs");
    cpp.files(sources("native", "cpp")).includes(&includes);
    if msvc {
        // windows.h min/max macros break std::min/std::max
        cpp.define("NOMINMAX", None)
            .define("WIN32_LEAN_AND_MEAN", None);
        cpp.flag("/std:c++20").flag("/EHsc").flag("/MD");
    } else {
        cpp.flag("-std=c++20");
        cpp.flag("-Wno-unused-parameter");
    }
    if release {
        cpp.define("NDEBUG", None);
    }
    if logging {
        cpp.define("LOGGING_ENABLED", None);
    }
    cpp.compile("mhttp");

    // tests that talk to real servers need the engine
    println!("cargo::rustc-cfg=mhttp_engine");

    link_search(boringssl.join("build"));
    link_search(nghttp2.join("build/lib"));
    link_search(libuv.join("build"));
    link_search(cppcoro.join("build/lib"));

    println!("cargo:rustc-link-lib=static=ssl");
    println!("cargo:rustc-link-lib=static=crypto");
    println!("cargo:rustc-link-lib=static=nghttp2");
    println!("cargo:rustc-link-lib=static=cppcoro");

    if msvc {
        println!("cargo:rustc-link-lib=static=libuv");
        for lib in [
            "ws2_32", "iphlpapi", "userenv", "psapi", "user32", "advapi32", "dbghelp", "ole32",
            "shell32",
        ] {
            println!("cargo:rustc-link-lib={lib}");
        }
    } else {
        println!("cargo:rustc-link-lib=static=uv");
        println!("cargo:rustc-link-lib=pthread");
        println!("cargo:rustc-link-lib=dl");
    }

    // libuv's fs events on macos
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=framework=CoreServices");
    }
}
