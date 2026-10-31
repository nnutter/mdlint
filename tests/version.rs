use std::process::Command;

#[test]
fn version_matches_release_tag_or_git_description() {
    let expected = option_env!("MDLINT_RELEASE_TAG").map_or_else(
        || {
            let output = Command::new("git")
                .args(["describe", "--tags", "--long"])
                .output()
                .unwrap();
            assert!(output.status.success());
            String::from_utf8(output.stdout).unwrap().trim().to_owned()
        },
        str::to_owned,
    );
    let expected = expected.strip_prefix('v').unwrap_or(&expected);
    let output = Command::new(env!("CARGO_BIN_EXE_mdlint"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!("mdlint {expected}\n")
    );
}
