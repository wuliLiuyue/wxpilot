use std::env;
use std::sync::{Arc, Barrier, Mutex, OnceLock};
use std::thread;

use tempfile::tempdir;
use wxp_license::{
    derive_public_key_hex, verify_certificate, ActivationRequest, test_keys::PRIVATE_KEY_HEX,
};
use wxp_license_server::{activate, seed_license, ServerConfig, ServerError};

#[test]
fn activate_binds_first_machine_and_rejects_second_machine() {
    let temp = tempdir().expect("tempdir");
    let db_path = temp.path().join("license.db");
    let key = "lic-live-001";
    seed_license(&db_path, key).expect("seed license");

    let config = ServerConfig {
        bind_addr: "127.0.0.1:0".into(),
        database_path: db_path,
        private_key_hex: PRIVATE_KEY_HEX.into(),
        certificate_key_id: "dev".into(),
    };

    let first = activate(
        &config,
        ActivationRequest {
            license_key: key.into(),
            machine_fingerprint: "machine-a".into(),
            client_version: "0.1.0".into(),
        },
    )
    .expect("first activation");

    let public_key = derive_public_key_hex(PRIVATE_KEY_HEX).expect("derive public key");
    verify_certificate(&first, &public_key).expect("verify certificate");

    let same_machine = activate(
        &config,
        ActivationRequest {
            license_key: key.into(),
            machine_fingerprint: "machine-a".into(),
            client_version: "0.1.0".into(),
        },
    )
    .expect("same machine reactivation");
    assert_eq!(
        same_machine.payload.machine_fingerprint,
        first.payload.machine_fingerprint
    );

    let second = activate(
        &config,
        ActivationRequest {
            license_key: key.into(),
            machine_fingerprint: "machine-b".into(),
            client_version: "0.1.0".into(),
        },
    )
    .expect_err("second machine should be rejected");
    assert!(matches!(second, ServerError::AlreadyBound));
}

#[test]
fn server_config_requires_explicit_signing_key() {
    let _guard = env_lock().lock().expect("lock env");
    let previous_key = env::var_os("WXP_LICENSE_PRIVATE_KEY_HEX");
    env::remove_var("WXP_LICENSE_PRIVATE_KEY_HEX");

    let result = ServerConfig::from_env();

    restore_env("WXP_LICENSE_PRIVATE_KEY_HEX", previous_key);
    assert!(matches!(
        result,
        Err(ServerError::MissingConfig(ref name)) if name == "WXP_LICENSE_PRIVATE_KEY_HEX"
    ));
}

#[test]
fn concurrent_activation_maps_second_bind_to_already_bound() {
    let temp = tempdir().expect("tempdir");
    let db_path = temp.path().join("license.db");
    let key = "lic-live-001";
    seed_license(&db_path, key).expect("seed license");

    let config = Arc::new(ServerConfig {
        bind_addr: "127.0.0.1:0".into(),
        database_path: db_path,
        private_key_hex: PRIVATE_KEY_HEX.into(),
        certificate_key_id: "dev".into(),
    });
    let barrier = Arc::new(Barrier::new(3));
    let mut handles = Vec::new();

    for machine in ["machine-a", "machine-b"] {
        let barrier = Arc::clone(&barrier);
        let config = Arc::clone(&config);
        let license_key = key.to_string();
        let machine = machine.to_string();
        handles.push(thread::spawn(move || {
            barrier.wait();
            activate(
                &config,
                ActivationRequest {
                    license_key,
                    machine_fingerprint: machine,
                    client_version: "0.1.0".into(),
                },
            )
        }));
    }

    barrier.wait();
    let results = handles
        .into_iter()
        .map(|handle| handle.join().expect("join worker"))
        .collect::<Vec<_>>();
    let success_count = results.iter().filter(|result| result.is_ok()).count();
    let already_bound_count = results
        .iter()
        .filter(|result| matches!(result, Err(ServerError::AlreadyBound)))
        .count();

    assert_eq!(success_count, 1, "expected exactly one successful bind");
    assert_eq!(
        already_bound_count, 1,
        "expected competing bind to map to AlreadyBound"
    );
}

fn env_lock() -> &'static Mutex<()> {
    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    ENV_LOCK.get_or_init(|| Mutex::new(()))
}

fn restore_env(key: &str, value: Option<std::ffi::OsString>) {
    match value {
        Some(value) => env::set_var(key, value),
        None => env::remove_var(key),
    }
}
