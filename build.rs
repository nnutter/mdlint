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

fn main() {
    for path in ["HEAD", "refs", "packed-refs"] {
        println!(
            "cargo:rerun-if-changed={}",
            git(&["rev-parse", "--git-path", path])
        );
    }
    println!("cargo:rerun-if-env-changed=MDLINT_RELEASE_TAG");
    let description = git(&["describe", "--tags", "--long"]);
    let version = std::env::var("MDLINT_RELEASE_TAG").unwrap_or(description);
    println!(
        "cargo:rustc-env=MDLINT_VERSION={}",
        version.strip_prefix('v').unwrap_or(&version)
    );
}
