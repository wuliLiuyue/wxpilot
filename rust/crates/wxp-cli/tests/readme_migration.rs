use std::fs;

#[test]
fn readme_uses_rust_cli_as_primary_guide() {
    let readme = fs::read_to_string("../../../README.md").expect("read README");
    assert!(readme.contains("cargo build -p wxp-cli --bin wxpilot"));
    assert!(readme.contains("cargo build -p wxp-daemon --bin wxp-daemon"));
    assert!(readme.contains("Rust CLI / Rust daemon 为默认实现"));
    assert!(readme.contains("wxpilot start <projectPath> --proxy"));
}
