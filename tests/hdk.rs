use assert_cmd::Command;

#[test]
fn basic_debug_build() {
    let mut cmd = Command::cargo_bin("cargo-hdk").unwrap();
    cmd.arg("--hdk-path").arg("./tests/hdk").assert().success();
}
