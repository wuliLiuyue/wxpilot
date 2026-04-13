use std::fs;

#[test]
fn readme_uses_curl_install_as_primary_guide() {
    let readme = fs::read_to_string("../../../README.md").expect("read README");
    assert!(readme.contains(
        "curl -fsSL https://raw.githubusercontent.com/wuliLiuyue/wxpilot/main/install.sh | sh"
    ));
    assert!(readme.contains("~/.local/bin/wxpilot"));
    assert!(readme.contains("~/.wxpilot/bin/wxp-daemon"));
    assert!(readme.contains("### 开发 / 从源码运行"));
    assert!(readme.contains("Rust CLI / Rust daemon 为默认实现"));
    assert!(readme.contains("wxpilot start <projectPath> --proxy"));
}
