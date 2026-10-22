use std::process::Command;

fn main() {
    // The rustc version labels the runtime (platform_string and the CI
    // filename share it), captured at build time from the same toolchain
    // cargo is driving.
    if let Ok(out) = Command::new("rustc").arg("--version").output() {
        let v = String::from_utf8_lossy(&out.stdout);
        if let Some(ver) = v.split_whitespace().nth(1) {
            println!("cargo:rustc-env=SERIALBENCH_RUSTC_VERSION={ver}");
            return;
        }
    }
    println!("cargo:rustc-env=SERIALBENCH_RUSTC_VERSION=unknown");
}
