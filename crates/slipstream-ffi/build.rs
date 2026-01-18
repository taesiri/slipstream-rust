use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-env-changed=PICOQUIC_DIR");
    println!("cargo:rerun-if-env-changed=PICOQUIC_BUILD_DIR");
    println!("cargo:rerun-if-env-changed=PICOQUIC_INCLUDE_DIR");
    println!("cargo:rerun-if-env-changed=PICOQUIC_LIB_DIR");
    println!("cargo:rerun-if-env-changed=PICOQUIC_AUTO_BUILD");
    println!("cargo:rerun-if-env-changed=PICOTLS_INCLUDE_DIR");

    let explicit_paths = has_explicit_picoquic_paths();
    let auto_build = env_flag("PICOQUIC_AUTO_BUILD", true);
    let mut picoquic_include_dir = locate_picoquic_include_dir();
    let mut picoquic_lib_dir = locate_picoquic_lib_dir();
    let mut picotls_include_dir = locate_picotls_include_dir();

    if auto_build
        && !explicit_paths
        && (picoquic_include_dir.is_none() || picoquic_lib_dir.is_none())
    {
        println!("cargo:warning=auto-building picoquic (set PICOQUIC_AUTO_BUILD=0 to disable)");
        build_picoquic()?;
        picoquic_include_dir = locate_picoquic_include_dir();
        picoquic_lib_dir = locate_picoquic_lib_dir();
        picotls_include_dir = locate_picotls_include_dir();
    }

    let picoquic_include_dir = picoquic_include_dir.ok_or(
        "Missing picoquic headers; set PICOQUIC_DIR or PICOQUIC_INCLUDE_DIR (default: vendor/picoquic).",
    )?;
    let picoquic_lib_dir = picoquic_lib_dir.ok_or(
        "Missing picoquic build artifacts; run ./scripts/build_picoquic.sh or set PICOQUIC_BUILD_DIR/PICOQUIC_LIB_DIR.",
    )?;
    let picotls_include_dir = picotls_include_dir.ok_or(
        "Missing picotls headers; set PICOTLS_INCLUDE_DIR or build picoquic with PICOQUIC_FETCH_PTLS=ON.",
    )?;

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let cc_dir = manifest_dir.join("cc");
    let cc_src = cc_dir.join("slipstream_server_cc.c");
    let mixed_cc_src = cc_dir.join("slipstream_mixed_cc.c");
    let poll_src = cc_dir.join("slipstream_poll.c");
    let test_helpers_src = cc_dir.join("slipstream_test_helpers.c");
    let picotls_layout_src = cc_dir.join("picotls_layout.c");
    let wincompat_time_src = cc_dir.join("wincompat_time.c");
    println!("cargo:rerun-if-changed={}", cc_src.display());
    println!("cargo:rerun-if-changed={}", mixed_cc_src.display());
    println!("cargo:rerun-if-changed={}", poll_src.display());
    println!("cargo:rerun-if-changed={}", test_helpers_src.display());
    println!("cargo:rerun-if-changed={}", picotls_layout_src.display());
    println!("cargo:rerun-if-changed={}", wincompat_time_src.display());
    let picoquic_internal = picoquic_include_dir.join("picoquic_internal.h");
    if picoquic_internal.exists() {
        println!("cargo:rerun-if-changed={}", picoquic_internal.display());
    }
    let mut cc_build = cc::Build::new();
    if cfg!(windows) {
        cc_build.define("_WINDOWS", None);
        cc_build.define("WIN32", None);
    }
    cc_build
        .include(&picoquic_include_dir)
        .include(&picotls_include_dir)
        .file(&cc_src)
        .file(&mixed_cc_src)
        .file(&poll_src)
        .file(&test_helpers_src)
        .file(&picotls_layout_src)
        .flag_if_supported("-fPIC");
    if cfg!(windows) {
        cc_build.file(&wincompat_time_src);
    }
    cc_build.compile("slipstream_client_objs");

    let picoquic_libs = resolve_picoquic_libs(&picoquic_lib_dir).ok_or(
        "Missing picoquic build artifacts; run ./scripts/build_picoquic.sh or set PICOQUIC_BUILD_DIR/PICOQUIC_LIB_DIR.",
    )?;
    for dir in picoquic_libs.search_dirs {
        println!("cargo:rustc-link-search=native={}", dir.display());
    }
    for lib in picoquic_libs.libs {
        println!("cargo:rustc-link-lib=static={}", lib);
    }

    if !cfg!(windows) {
        println!("cargo:rustc-link-lib=dylib=ssl");
        println!("cargo:rustc-link-lib=dylib=crypto");
        println!("cargo:rustc-link-lib=dylib=pthread");
    }

    Ok(())
}

fn locate_repo_root() -> Option<PathBuf> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").ok()?;
    let crate_dir = Path::new(&manifest_dir);
    Some(crate_dir.parent()?.parent()?.to_path_buf())
}

fn env_flag(key: &str, default: bool) -> bool {
    match env::var(key) {
        Ok(value) => {
            let value = value.trim().to_ascii_lowercase();
            matches!(value.as_str(), "1" | "true" | "yes" | "on")
        }
        Err(_) => default,
    }
}

fn has_explicit_picoquic_paths() -> bool {
    env::var_os("PICOQUIC_DIR").is_some()
        || env::var_os("PICOQUIC_INCLUDE_DIR").is_some()
        || env::var_os("PICOQUIC_BUILD_DIR").is_some()
        || env::var_os("PICOQUIC_LIB_DIR").is_some()
}

fn build_picoquic() -> Result<(), Box<dyn std::error::Error>> {
    let root = locate_repo_root().ok_or("Could not locate repository root for picoquic build")?;
    let script = root.join("scripts").join("build_picoquic.sh");
    if !script.exists() {
        return Err("scripts/build_picoquic.sh not found; run git submodule update --init --recursive vendor/picoquic".into());
    }
    let picoquic_dir = env::var_os("PICOQUIC_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("vendor").join("picoquic"));
    if !picoquic_dir.exists() {
        return Err("picoquic submodule missing; run git submodule update --init --recursive vendor/picoquic".into());
    }
    let build_dir = env::var_os("PICOQUIC_BUILD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(".picoquic-build"));

    if cfg!(windows) {
        return build_picoquic_windows(&picoquic_dir, &build_dir);
    }

    let status = Command::new(&script)
        .current_dir(&root)
        .env("PICOQUIC_DIR", picoquic_dir)
        .env("PICOQUIC_BUILD_DIR", build_dir)
        .status()?;
    if !status.success() {
        return Err(
            "picoquic auto-build failed (run scripts/build_picoquic.sh for details)".into(),
        );
    }
    Ok(())
}

fn find_cmake_exe() -> Option<PathBuf> {
    if let Ok(dir) = env::var("CMAKE") {
        let candidate = PathBuf::from(dir);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    let program_files = [
        "C:\\Program Files\\CMake\\bin\\cmake.exe",
        "C:\\Program Files (x86)\\CMake\\bin\\cmake.exe",
    ];
    for path in program_files {
        let candidate = PathBuf::from(path);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    if let Some(path_var) = env::var_os("PATH") {
        for entry in env::split_paths(&path_var) {
            let candidate = entry.join("cmake.exe");
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    None
}

fn find_pkgconf_exe() -> Option<PathBuf> {
    if let Ok(dir) = env::var("PKG_CONFIG_EXECUTABLE") {
        let candidate = PathBuf::from(dir);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    if let Ok(dir) = env::var("PKG_CONFIG") {
        let candidate = PathBuf::from(dir);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    if let Ok(root) = env::var("VCPKG_ROOT") {
        let tools_dir = Path::new(&root).join("downloads").join("tools").join("msys2");
        if let Ok(entries) = fs::read_dir(&tools_dir) {
            for entry in entries.flatten() {
                let base = entry.path();
                let mingw64 = base.join("mingw64").join("bin").join("pkgconf.exe");
                if mingw64.exists() {
                    return Some(mingw64);
                }
                let usr = base.join("usr").join("bin").join("pkgconf.exe");
                if usr.exists() {
                    return Some(usr);
                }
            }
        }
    }

    if let Some(path_var) = env::var_os("PATH") {
        for entry in env::split_paths(&path_var) {
            let pkgconf = entry.join("pkgconf.exe");
            if pkgconf.exists() {
                return Some(pkgconf);
            }
            let pkgconfig = entry.join("pkg-config.exe");
            if pkgconfig.exists() {
                return Some(pkgconfig);
            }
        }
    }

    None
}

fn build_picoquic_windows(
    picoquic_dir: &Path,
    build_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let cmake = find_cmake_exe().unwrap_or_else(|| PathBuf::from("cmake"));
    let build_type = env::var("BUILD_TYPE").unwrap_or_else(|_| "Release".to_string());
    let fetch_ptls = env::var("PICOQUIC_FETCH_PTLS").unwrap_or_else(|_| "ON".to_string());
    let pkgconf = find_pkgconf_exe();
    let openssl_root = env::var("VCPKG_ROOT")
        .ok()
        .map(|root| Path::new(&root).join("installed").join("x64-windows-static-md"));

    let mut configure = Command::new(&cmake);
    configure
        .arg("-S")
        .arg(picoquic_dir)
        .arg("-B")
        .arg(build_dir)
        .arg(format!("-DCMAKE_BUILD_TYPE={}", build_type))
        .arg(format!("-DPICOQUIC_FETCH_PTLS={}", fetch_ptls))
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON");
    if let Some(pkgconf) = pkgconf {
        configure.arg(format!(
            "-DPKG_CONFIG_EXECUTABLE={}",
            pkgconf.display()
        ));
    }
    if let Some(root) = openssl_root.as_ref().filter(|dir| dir.exists()) {
        configure
            .arg(format!("-DOPENSSL_ROOT_DIR={}", root.display()))
            .arg("-DOPENSSL_USE_STATIC_LIBS=ON");
    }
    let status = configure.status()?;
    if !status.success() {
        return Err("picoquic CMake configure failed".into());
    }

    let picotls_include = build_dir
        .join("_deps")
        .join("picotls-src")
        .join("include");
    let wincompat_src = picoquic_dir.join("picoquic").join("wincompat.h");
    if picotls_include.exists() && wincompat_src.exists() {
        let wincompat_dst = picotls_include.join("wincompat.h");
        fs::copy(&wincompat_src, &wincompat_dst)?;
    }

    let status = Command::new(&cmake)
        .arg("--build")
        .arg(build_dir)
        .arg("--target")
        .arg("picoquic-core")
        .arg("picotls-core")
        .arg("picotls-openssl")
        .arg("picotls-fusion")
        .arg("picotls-minicrypto")
        .status()?;
    if !status.success() {
        return Err("picoquic CMake build failed".into());
    }

    Ok(())
}

fn locate_picoquic_include_dir() -> Option<PathBuf> {
    if let Ok(dir) = env::var("PICOQUIC_INCLUDE_DIR") {
        let candidate = PathBuf::from(dir);
        if has_picoquic_internal_header(&candidate) {
            return Some(candidate);
        }
    }

    if let Ok(dir) = env::var("PICOQUIC_DIR") {
        let candidate = PathBuf::from(&dir);
        if has_picoquic_internal_header(&candidate) {
            return Some(candidate);
        }
        let candidate = Path::new(&dir).join("picoquic");
        if has_picoquic_internal_header(&candidate) {
            return Some(candidate);
        }
    }

    if let Some(root) = locate_repo_root() {
        let candidate = root.join("vendor").join("picoquic").join("picoquic");
        if has_picoquic_internal_header(&candidate) {
            return Some(candidate);
        }
    }

    None
}

fn locate_picoquic_lib_dir() -> Option<PathBuf> {
    if let Ok(dir) = env::var("PICOQUIC_LIB_DIR") {
        let candidate = PathBuf::from(dir);
        if has_picoquic_libs(&candidate) {
            return Some(candidate);
        }
    }

    if let Ok(dir) = env::var("PICOQUIC_BUILD_DIR") {
        let candidate = PathBuf::from(&dir);
        if has_picoquic_libs(&candidate) {
            return Some(candidate);
        }
        let candidate = Path::new(&dir).join("picoquic");
        if has_picoquic_libs(&candidate) {
            return Some(candidate);
        }
    }

    if let Some(root) = locate_repo_root() {
        let candidate = root.join(".picoquic-build");
        if has_picoquic_libs(&candidate) {
            return Some(candidate);
        }
        let candidate = root.join(".picoquic-build").join("picoquic");
        if has_picoquic_libs(&candidate) {
            return Some(candidate);
        }
    }

    None
}

fn locate_picotls_include_dir() -> Option<PathBuf> {
    if let Ok(dir) = env::var("PICOTLS_INCLUDE_DIR") {
        let candidate = PathBuf::from(dir);
        if has_picotls_header(&candidate) {
            return Some(candidate);
        }
    }

    if let Ok(dir) = env::var("PICOQUIC_BUILD_DIR") {
        let candidate = Path::new(&dir)
            .join("_deps")
            .join("picotls-src")
            .join("include");
        if has_picotls_header(&candidate) {
            return Some(candidate);
        }
    }

    if let Ok(dir) = env::var("PICOQUIC_LIB_DIR") {
        let candidate = Path::new(&dir)
            .join("_deps")
            .join("picotls-src")
            .join("include");
        if has_picotls_header(&candidate) {
            return Some(candidate);
        }
        if let Some(parent) = Path::new(&dir).parent() {
            let candidate = parent.join("_deps").join("picotls-src").join("include");
            if has_picotls_header(&candidate) {
                return Some(candidate);
            }
        }
    }

    if let Some(root) = locate_repo_root() {
        let candidate = root
            .join(".picoquic-build")
            .join("_deps")
            .join("picotls-src")
            .join("include");
        if has_picotls_header(&candidate) {
            return Some(candidate);
        }
        let candidate = root
            .join("vendor")
            .join("picoquic")
            .join("picotls")
            .join("include");
        if has_picotls_header(&candidate) {
            return Some(candidate);
        }
    }

    None
}

fn has_picoquic_internal_header(dir: &Path) -> bool {
    dir.join("picoquic_internal.h").exists()
}

fn has_picotls_header(dir: &Path) -> bool {
    dir.join("picotls.h").exists()
}

fn has_picoquic_libs(dir: &Path) -> bool {
    resolve_picoquic_libs(dir).is_some()
}

struct PicoquicLibs {
    search_dirs: Vec<PathBuf>,
    libs: Vec<&'static str>,
}

fn resolve_picoquic_libs(dir: &Path) -> Option<PicoquicLibs> {
    for candidate in candidate_lib_dirs(dir) {
        if let Some(libs) = resolve_picoquic_libs_single_dir(&candidate) {
            return Some(PicoquicLibs {
                search_dirs: vec![candidate],
                libs,
            });
        }
    }

    let mut picotls_dirs = vec![dir.join("_deps").join("picotls-build")];
    if let Some(parent) = dir.parent() {
        picotls_dirs.push(parent.join("_deps").join("picotls-build"));
    }
    for picotls_dir in picotls_dirs {
        for picoquic_dir in candidate_lib_dirs(dir) {
            for picotls_candidate in candidate_lib_dirs(&picotls_dir) {
                if let Some(libs) = resolve_picoquic_libs_split(&picoquic_dir, &picotls_candidate)
                {
                    let mut search_dirs = vec![picoquic_dir];
                    if picotls_candidate != search_dirs[0]
                        && !search_dirs.contains(&picotls_candidate)
                    {
                        search_dirs.push(picotls_candidate);
                    }
                    return Some(PicoquicLibs { search_dirs, libs });
                }
            }
        }
    }

    if let Some(parent) = dir.parent() {
        for picoquic_dir in candidate_lib_dirs(parent) {
            for picotls_dir in candidate_lib_dirs(dir) {
                if let Some(libs) = resolve_picoquic_libs_split(&picoquic_dir, &picotls_dir) {
                    return Some(PicoquicLibs {
                        search_dirs: vec![picoquic_dir, picotls_dir],
                        libs,
                    });
                }
            }
        }
        if let Some(grandparent) = parent.parent() {
            for picoquic_dir in candidate_lib_dirs(grandparent) {
                for picotls_dir in candidate_lib_dirs(dir) {
                    if let Some(libs) = resolve_picoquic_libs_split(&picoquic_dir, &picotls_dir)
                    {
                        return Some(PicoquicLibs {
                            search_dirs: vec![picoquic_dir, picotls_dir],
                            libs,
                        });
                    }
                }
            }
        }
    }

    None
}

fn candidate_lib_dirs(dir: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![dir.to_path_buf()];
    if cfg!(windows) {
        dirs.push(dir.join("Debug"));
        dirs.push(dir.join("Release"));
    }
    dirs
}

fn resolve_picoquic_libs_single_dir(dir: &Path) -> Option<Vec<&'static str>> {
    const REQUIRED: [(&str, &str); 5] = [
        ("picoquic_core", "picoquic-core"),
        ("picotls_core", "picotls-core"),
        ("picotls_fusion", "picotls-fusion"),
        ("picotls_minicrypto", "picotls-minicrypto"),
        ("picotls_openssl", "picotls-openssl"),
    ];
    let mut libs = Vec::with_capacity(REQUIRED.len());
    for (underscored, hyphenated) in REQUIRED {
        libs.push(find_lib_variant(dir, underscored, hyphenated)?);
    }
    Some(libs)
}

fn resolve_picoquic_libs_split(
    picoquic_dir: &Path,
    picotls_dir: &Path,
) -> Option<Vec<&'static str>> {
    let picoquic_core = find_lib_variant(picoquic_dir, "picoquic_core", "picoquic-core")?;
    let picotls_core = find_lib_variant(picotls_dir, "picotls_core", "picotls-core")?;
    let picotls_fusion = find_lib_variant(picotls_dir, "picotls_fusion", "picotls-fusion")?;
    let picotls_minicrypto =
        find_lib_variant(picotls_dir, "picotls_minicrypto", "picotls-minicrypto")?;
    let picotls_openssl = find_lib_variant(picotls_dir, "picotls_openssl", "picotls-openssl")?;
    Some(vec![
        picoquic_core,
        picotls_core,
        picotls_fusion,
        picotls_minicrypto,
        picotls_openssl,
    ])
}

fn find_lib_variant<'a>(dir: &Path, underscored: &'a str, hyphenated: &'a str) -> Option<&'a str> {
    if cfg!(windows) {
        let underscored_path = dir.join(format!("{}.lib", underscored));
        if underscored_path.exists() {
            return Some(underscored);
        }
        let hyphen_path = dir.join(format!("{}.lib", hyphenated));
        if hyphen_path.exists() {
            return Some(hyphenated);
        }
    }

    let underscored_path = dir.join(format!("lib{}.a", underscored));
    if underscored_path.exists() {
        return Some(underscored);
    }
    let hyphen_path = dir.join(format!("lib{}.a", hyphenated));
    if hyphen_path.exists() {
        return Some(hyphenated);
    }
    None
}

