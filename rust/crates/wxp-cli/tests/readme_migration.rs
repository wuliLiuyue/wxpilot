use std::fs;

#[test]
fn readme_adds_rust_cli_build_and_run_guide() {
    let readme = fs::read_to_string("../../../README.md").expect("read README");
    assert!(readme.contains("cargo build -p wxp-cli --bin wxpilot"));
    assert!(readme.contains("cargo run -p wxp-cli --bin wxpilot -- --help"));
    assert!(readme.contains("TypeScript CLI / daemon"));
}
