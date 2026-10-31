use std::process::Command;

fn git(args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .output()
        .expect("Git is required to build mdlint");
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr).trim()
    );
    String::from_utf8(output.stdout)
        .expect("Git output must be UTF-8")
        .trim()
        .to_owned()
}

/// The release tag that triggered this build, when one did.
///
/// A release build needs no Git repository at all: the release jobs build
/// inside containers where Git refuses a workspace it does not own, and a
/// version must not depend on Git being usable there.
fn release_tag() -> Option<String> {
    std::env::var("MDLINT_RELEASE_TAG").ok()
}

/// The nearest tag and commit, for a build made from a working copy.
fn git_description() -> String {
    for path in ["HEAD", "refs", "packed-refs"] {
        println!(
            "cargo:rerun-if-changed={}",
            git(&["rev-parse", "--git-path", path])
        );
    }
    git(&["describe", "--tags", "--long"])
}

fn main() {
    println!("cargo:rerun-if-env-changed=MDLINT_RELEASE_TAG");
    let version = release_tag().unwrap_or_else(git_description);
    println!(
        "cargo:rustc-env=MDLINT_VERSION={}",
        version.strip_prefix('v').unwrap_or(&version)
    );
}
