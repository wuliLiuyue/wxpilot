use tempfile::tempdir;
use wxp_license::{
    derive_public_key_hex, verify_certificate, ActivationRequest, DEFAULT_DEV_PRIVATE_KEY_HEX,
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
        private_key_hex: DEFAULT_DEV_PRIVATE_KEY_HEX.into(),
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

    let public_key = derive_public_key_hex(DEFAULT_DEV_PRIVATE_KEY_HEX).expect("derive public key");
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
