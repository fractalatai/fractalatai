//! Stamp the git commit into `FRACTALAW_VERSION` for enrichment provenance
//! (fractalatai #63). Falls back to the crate version outside a git checkout.

use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn main() {
    let version = git(&["rev-parse", "--short=12", "HEAD"])
        .unwrap_or_else(|| format!("v{}", std::env::var("CARGO_PKG_VERSION").unwrap_or_default()));
    println!("cargo:rustc-env=FRACTALAW_VERSION={version}");

    // Re-stamp when HEAD moves (checkout, commit)
    if let Some(git_dir) = git(&["rev-parse", "--absolute-git-dir"]) {
        println!("cargo:rerun-if-changed={git_dir}/HEAD");
        println!("cargo:rerun-if-changed={git_dir}/packed-refs");
        if let Some(head_ref) = git(&["symbolic-ref", "-q", "HEAD"]) {
            println!("cargo:rerun-if-changed={git_dir}/{head_ref}");
        }
    }
}
