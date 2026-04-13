use wxp_daemon::bootstrap_guard::{run_bootstrap_guard, AllowAllBootstrapGuard};
use wxp_daemon::build_probe_payload;
use wxp_daemon::handlers::handle_probe;

#[test]
fn health_probe_exposes_rust_daemon_kind() {
    let payload = build_probe_payload();
    assert_eq!(payload.daemon_kind, "rust");
    assert_eq!(payload.status, "ok");
}

#[test]
fn health_version_capabilities_are_available() {
    assert_eq!(handle_probe("health")["status"], "ok");
    assert_eq!(handle_probe("version")["daemonKind"], "rust");
    assert_eq!(handle_probe("capabilities")["find"], true);
}

#[test]
fn health_version_capabilities_publish_public_method_manifest() {
    let capabilities = handle_probe("capabilities");
    let methods = capabilities["methods"].as_array().expect("methods array");

    assert_eq!(capabilities["status"], "ok");
    assert_eq!(capabilities["daemonKind"], "rust");
    assert!(methods.iter().any(|item| item == "state"));
    assert!(methods.iter().any(|item| item == "shot"));
    assert!(methods.iter().any(|item| item == "run"));
    assert!(methods.iter().any(|item| item == "wx"));
    assert!(methods.iter().any(|item| item == "net.log"));
}

#[test]
fn bootstrap_guard_defaults_to_allow_for_current_rollout() {
    let decision = run_bootstrap_guard(&AllowAllBootstrapGuard);

    assert!(decision.allowed);
    assert_eq!(decision.reason, None);
}
