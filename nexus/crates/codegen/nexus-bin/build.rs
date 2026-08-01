use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-env-changed=NEXUS_VERSION");
    println!("cargo:rerun-if-changed=nexus.rc");
    println!("cargo:rerun-if-changed=nexus.ico");

    // Windows: reserve a 16 MiB main-thread stack. Debug builds give the
    // startup async state machines (e.g. `async_main`'s generated poll) huge
    // stack frames — the MSVC default 1 MiB main-thread stack overflows
    // ("thread 'main' has overflowed its stack") on first launch.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        println!("cargo:rustc-link-arg-bins=/STACK:16777216");
    }

    // Windows: embed the Nexus icon into the .exe by compiling nexus.rc → nexus.res
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        if let Some(rc_path) = find_rc_exe() {
            let out_dir = std::env::var("OUT_DIR").unwrap();
            let res_path = format!("{out_dir}/nexus.res");
            let result = Command::new(&rc_path)
                .args(["/nologo", "/fo", &res_path, "nexus.rc"])
                .output();
            match result {
                Ok(o) if o.status.success() => {
                    println!("cargo:rustc-link-arg={res_path}");
                    println!("cargo:warning=icon embedded successfully");
                }
                Ok(o) => {
                    let stderr = String::from_utf8_lossy(&o.stderr);
                    println!("cargo:warning=rc.exe failed: {stderr}");
                }
                Err(e) => {
                    println!("cargo:warning=rc.exe spawn failed: {e}");
                }
            }
        } else {
            println!("cargo:warning=rc.exe not found, icon not embedded");
        }
    }

    let commit = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let version = std::env::var("NEXUS_VERSION")
        .or_else(|_| std::env::var("CARGO_PKG_VERSION"))
        .unwrap_or_else(|_| "0.0.0".to_string());

    println!(
        "cargo:rustc-env=VERSION_WITH_COMMIT={} ({})",
        version, commit
    );
}

/// Locate `rc.exe` by scanning the Windows Kits directory dynamically.
/// Does NOT hardcode specific SDK version numbers.
fn find_rc_exe() -> Option<std::path::PathBuf> {
    let kits_bin = Path::new(r"C:\Program Files (x86)\Windows Kits\10\bin");
    if !kits_bin.exists() {
        return None;
    }
    let entries = match std::fs::read_dir(kits_bin) {
        Ok(e) => e,
        Err(_) => return None,
    };
    for entry in entries.flatten() {
        let rc = entry.path().join("x64").join("rc.exe");
        if rc.exists() {
            return Some(rc);
        }
    }
    None
}
