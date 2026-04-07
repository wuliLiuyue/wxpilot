use std::fs;

use tempfile::tempdir;
use wxp_cli::project_detect::{detect_or_select_project, DetectProjectError, Selector};

#[test]
fn finds_single_dist_candidate_within_two_levels() {
    let temp = tempdir().expect("tempdir");
    let dist = temp.path().join("demo").join("dist");
    fs::create_dir_all(&dist).expect("mkdir");
    fs::write(dist.join("project.config.json"), "{}").expect("write config");

    let result = detect_or_select_project(temp.path(), false, Selector::fixed(0)).expect("detect");
    assert_eq!(result, dist);
}

#[test]
fn finds_root_dist_candidate() {
    let temp = tempdir().expect("tempdir");
    let dist = temp.path().join("dist");
    fs::create_dir_all(&dist).expect("mkdir");
    fs::write(dist.join("project.config.json"), "{}").expect("write config");

    let result = detect_or_select_project(temp.path(), false, Selector::fixed(0)).expect("detect");
    assert_eq!(result, dist);
}

#[test]
fn finds_single_dist_candidate_within_two_subdirectory_levels() {
    let temp = tempdir().expect("tempdir");
    let dist = temp.path().join("demo").join("nested").join("dist");
    fs::create_dir_all(&dist).expect("mkdir");
    fs::write(dist.join("project.config.json"), "{}").expect("write config");

    let result = detect_or_select_project(temp.path(), false, Selector::fixed(0)).expect("detect");
    assert_eq!(result, dist);
}

#[test]
fn ignores_dist_beyond_two_subdirectory_levels() {
    let temp = tempdir().expect("tempdir");
    let dist = temp.path().join("demo").join("nested").join("deeper").join("dist");
    fs::create_dir_all(&dist).expect("mkdir");
    fs::write(dist.join("project.config.json"), "{}").expect("write config");

    let error =
        detect_or_select_project(temp.path(), false, Selector::fixed(0)).expect_err("too deep should not match");
    assert!(matches!(error, DetectProjectError::NotFound));
}

#[test]
fn returns_explicit_error_when_multiple_candidates_and_not_tty() {
    let temp = tempdir().expect("tempdir");
    for name in ["a", "b"] {
        let dist = temp.path().join(name).join("dist");
        fs::create_dir_all(&dist).expect("mkdir");
        fs::write(dist.join("project.config.json"), "{}").expect("write config");
    }

    let error =
        detect_or_select_project(temp.path(), false, Selector::fixed(0)).expect_err("non tty should fail");
    assert!(matches!(error, DetectProjectError::MultipleCandidates { .. }));
    assert!(error.to_string().contains("未找到唯一项目路径"));
}

#[test]
fn uses_selector_when_multiple_candidates_and_tty() {
    let temp = tempdir().expect("tempdir");
    let first = temp.path().join("demo-a").join("dist");
    let second = temp.path().join("demo-b").join("dist");
    for dist in [&first, &second] {
        fs::create_dir_all(dist).expect("mkdir");
        fs::write(dist.join("project.config.json"), "{}").expect("write config");
    }

    let result = detect_or_select_project(temp.path(), true, Selector::fixed(1)).expect("select");
    assert_eq!(result, second);
}

#[test]
fn uses_custom_selector_when_multiple_candidates_and_tty() {
    let temp = tempdir().expect("tempdir");
    let first = temp.path().join("demo-a").join("dist");
    let second = temp.path().join("demo-b").join("dist");
    for dist in [&first, &second] {
        fs::create_dir_all(dist).expect("mkdir");
        fs::write(dist.join("project.config.json"), "{}").expect("write config");
    }

    let selector = Selector::new(|_| Ok(1));
    let result = detect_or_select_project(temp.path(), true, selector).expect("select");
    assert_eq!(result, second);
}

#[test]
fn returns_selection_failed_when_selector_errors() {
    let temp = tempdir().expect("tempdir");
    let first = temp.path().join("demo-a").join("dist");
    let second = temp.path().join("demo-b").join("dist");
    for dist in [&first, &second] {
        fs::create_dir_all(dist).expect("mkdir");
        fs::write(dist.join("project.config.json"), "{}").expect("write config");
    }

    let selector = Selector::new(|_| {
        Err(DetectProjectError::SelectionFailed {
            message: "user cancelled".to_string(),
        })
    });
    let error = detect_or_select_project(temp.path(), true, selector).expect_err("selection should fail");
    assert!(matches!(error, DetectProjectError::SelectionFailed { .. }));
    assert!(error.to_string().contains("user cancelled"));
}

#[test]
fn returns_selection_failed_when_selector_returns_out_of_bounds_index() {
    let temp = tempdir().expect("tempdir");
    let first = temp.path().join("demo-a").join("dist");
    let second = temp.path().join("demo-b").join("dist");
    for dist in [&first, &second] {
        fs::create_dir_all(dist).expect("mkdir");
        fs::write(dist.join("project.config.json"), "{}").expect("write config");
    }

    let selector = Selector::new(|_| Ok(99));
    let error = detect_or_select_project(temp.path(), true, selector).expect_err("out of bounds should fail");
    assert!(matches!(error, DetectProjectError::SelectionFailed { .. }));
    assert!(error.to_string().contains("invalid index"));
}

#[cfg(unix)]
#[test]
fn returns_scan_error_when_project_config_check_hits_permission_error() {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempdir().expect("tempdir");
    let dist = temp.path().join("demo").join("dist");
    fs::create_dir_all(&dist).expect("mkdir");
    let mut perms = fs::metadata(&dist).expect("metadata").permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&dist, perms).expect("chmod 000");

    let error = detect_or_select_project(temp.path(), false, Selector::fixed(0))
        .expect_err("permission failure should surface as scan error");

    let mut restore = fs::metadata(&dist).expect("metadata restore").permissions();
    restore.set_mode(0o755);
    fs::set_permissions(&dist, restore).expect("chmod restore");

    assert!(matches!(error, DetectProjectError::ScanFailed { .. }));
}

#[test]
fn returns_scan_error_when_root_directory_is_missing() {
    let temp = tempdir().expect("tempdir");
    let root = temp.path().to_path_buf();
    drop(temp);

    let error = detect_or_select_project(&root, false, Selector::fixed(0))
        .expect_err("missing root should fail as scan error");
    assert!(matches!(error, DetectProjectError::ScanFailed { .. }));
    assert!(error.to_string().contains("扫描项目路径失败"));
}
