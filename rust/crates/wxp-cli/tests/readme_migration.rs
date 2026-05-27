use std::fs;

#[test]
fn readme_uses_rust_cli_as_primary_guide() {
    let readme = fs::read_to_string("../../../README.md").expect("read README");
    assert!(readme.contains("cargo build -p wxp-cli --bin wxpilot"));
    assert!(readme.contains("cargo build -p wxp-daemon --bin wxp-daemon"));
    assert!(readme.contains("Rust CLI / Rust daemon 为默认实现"));
    assert!(readme.contains("## 开发者本地打包与安装"));
    assert!(readme.contains("make local-build"));
    assert!(readme.contains("make local-install"));
    assert!(readme.contains("~/.local/bin/wxpilot"));
    assert!(readme.contains("~/.wxpilot/bin/wxp-daemon"));
    assert!(readme.contains("wxpilot start <projectPath> --proxy"));
    assert!(readme.contains("WXP_LICENSE_SERVER_URL"));
    assert!(readme.contains("make license-compose-up"));
    assert!(readme.contains("make license-init-test-env"));
    assert!(readme.contains("license-release-runbook.md"));
}
