use std::{
    env,
    fs,
    path::{Path, PathBuf},
};

fn copy_if_exists(src: &Path, dst_dir: &Path) -> Result<(), String> {
    if !src.exists() {
        return Err(format!("DLL not found: {}", src.display()));
    }
    let file_name = src
        .file_name()
        .ok_or_else(|| format!("Bad DLL path: {}", src.display()))?;
    let dst = dst_dir.join(file_name);
    fs::create_dir_all(dst_dir).map_err(|e| format!("create_dir_all failed: {e}"))?;
    fs::copy(src, &dst).map_err(|e| format!("copy {} -> {} failed: {e}", src.display(), dst.display()))?;
    Ok(())
}

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" {
        return; // no-op on non-Windows
    }

    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let subdir = match target_arch.as_str() {
        "x86_64" => "x64",
        "x86" => "x86",
        "aarch64" => "arm64",
        "arm" => "arm",
        other => {
            // Fail fast on unexpected architectures
            panic!("Unsupported target arch for CP210x DLL copy: {other}");
        }
    };

    // Project root is the directory containing Cargo.toml
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    // Where Cargo puts the final artifacts (.exe). OUT_DIR is under target/.../build/<crate>/out
    // Going up 3 levels from OUT_DIR usually lands in target/<profile>/ (the exe directory).
    // Example: target\debug\build\<crate-hash>\out -> target\debug
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let exe_dir = out_dir
        .ancestors()
        .nth(3)
        .expect("OUT_DIR doesn't have enough parents; Cargo layout changed?")
        .to_path_buf();

    let dll_dir = manifest_dir.join("dll").join(subdir);

    let dlls = ["CP210xManufacturing.dll", "CP210xRuntime.dll"];

    // Re-run build script if DLLs change
    println!("cargo:rerun-if-changed={}", dll_dir.display());
    for name in dlls {
        println!("cargo:rerun-if-changed={}", dll_dir.join(name).display());
    }

    for name in dlls {
        let src = dll_dir.join(name);
        copy_if_exists(&src, &exe_dir).unwrap_or_else(|e| panic!("{e}"));
    }

    // Optional: print where we copied for easier debugging in logs
    println!("cargo:warning=Copied CP210x DLLs from {} to {}", dll_dir.display(), exe_dir.display());
}
