use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use wxp_license::{
    verify_certificate, ActivationErrorResponse, ActivationRequest, ActivationResponse,
    SignedLicenseCertificate, DEFAULT_DEV_PUBLIC_KEY_HEX,
};

use crate::error::CliError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredLicense {
    pub certificate: SignedLicenseCertificate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivationResult {
    pub license_id: String,
    pub machine_fingerprint: String,
    pub path: String,
}

pub fn ensure_license() -> Result<(), CliError> {
    let path = license_path();
    if !path.exists() {
        return Err(CliError::LicenseRequired);
    }

    let stored = load_stored_license()?;
    verify_certificate(&stored.certificate, &license_public_key_hex())
        .map_err(|error| CliError::LicenseInvalid(error.to_string()))?;

    let current_machine = machine_fingerprint()?;
    if stored.certificate.payload.machine_fingerprint != current_machine {
        return Err(CliError::LicenseInvalid(
            "该凭证绑定到另一台机器，请使用当前机器重新购买/激活".into(),
        ));
    }

    Ok(())
}

pub fn activate_license(key: &str) -> Result<ActivationResult, CliError> {
    let key = key.trim();
    if key.is_empty() {
        return Err(CliError::ActivationFailed("license key 不能为空".into()));
    }

    let machine_fingerprint = machine_fingerprint()?;
    let client = ureq::post(&format!("{}/v1/activate", license_server_url()))
        .set("Content-Type", "application/json");
    let request = ActivationRequest {
        license_key: key.to_string(),
        machine_fingerprint: machine_fingerprint.clone(),
        client_version: env!("CARGO_PKG_VERSION").into(),
    };
    let response = client.send_json(ureq::json!(request));

    let certificate = match response {
        Ok(response) => {
            response
                .into_json::<ActivationResponse>()
                .map_err(|error| CliError::ActivationFailed(error.to_string()))?
                .certificate
        }
        Err(ureq::Error::Status(_, response)) => {
            let error = response
                .into_json::<ActivationErrorResponse>()
                .map(|payload| payload.message)
                .unwrap_or_else(|_| "服务器返回未知错误".into());
            return Err(CliError::ActivationFailed(error));
        }
        Err(error) => return Err(CliError::ActivationFailed(error.to_string())),
    };

    verify_certificate(&certificate, &license_public_key_hex()).map_err(|error| {
        CliError::ActivationFailed(format!("服务端返回的证书校验失败：{error}"))
    })?;

    let path = persist_certificate(certificate.clone())?;
    Ok(ActivationResult {
        license_id: certificate.payload.license_id,
        machine_fingerprint,
        path: path.display().to_string(),
    })
}

fn load_stored_license() -> Result<StoredLicense, CliError> {
    let raw = fs::read_to_string(license_path())
        .map_err(|error| CliError::LicenseInvalid(error.to_string()))?;
    serde_json::from_str(&raw).map_err(|error| CliError::LicenseInvalid(error.to_string()))
}

fn persist_certificate(certificate: SignedLicenseCertificate) -> Result<PathBuf, CliError> {
    let path = license_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| CliError::ActivationFailed(error.to_string()))?;
    }
    let stored = StoredLicense { certificate };
    let raw = serde_json::to_string_pretty(&stored)
        .map_err(|error| CliError::ActivationFailed(error.to_string()))?;
    fs::write(&path, raw).map_err(|error| CliError::ActivationFailed(error.to_string()))?;
    Ok(path)
}

fn license_path() -> PathBuf {
    env::var_os("WXP_LICENSE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| runtime_dir().join("license.json"))
}

fn license_server_url() -> String {
    env::var("WXP_LICENSE_SERVER_URL").unwrap_or_else(|_| "http://127.0.0.1:8787".into())
}

fn license_public_key_hex() -> String {
    env::var("WXP_LICENSE_PUBLIC_KEY_HEX").unwrap_or_else(|_| DEFAULT_DEV_PUBLIC_KEY_HEX.into())
}

fn runtime_dir() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".wxpilot")
}

fn machine_fingerprint() -> Result<String, CliError> {
    if let Some(value) = env::var_os("WXP_LICENSE_MACHINE_ID") {
        return Ok(hash_machine_identity(&value.to_string_lossy()));
    }

    let hostname = hostname().unwrap_or_else(|| "unknown-host".into());
    let user = env::var("USER").unwrap_or_else(|_| "unknown-user".into());
    let identity = format!(
        "host={hostname};user={user};os={};arch={}",
        env::consts::OS,
        env::consts::ARCH
    );
    Ok(hash_machine_identity(&identity))
}

fn hostname() -> Option<String> {
    let output = Command::new("hostname").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn hash_machine_identity(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::{hash_machine_identity, StoredLicense};
    use wxp_license::{sign_certificate, LicenseCertificate, DEFAULT_DEV_PRIVATE_KEY_HEX};

    #[test]
    fn machine_identity_is_hashed() {
        assert_ne!(hash_machine_identity("machine-a"), "machine-a");
    }

    #[test]
    fn stored_license_round_trips() {
        let payload = LicenseCertificate {
            version: 1,
            license_id: "lic_1".into(),
            key_id: "dev".into(),
            machine_fingerprint: hash_machine_identity("machine-a"),
            issued_at: "2026-04-13T00:00:00Z".into(),
            client_version: "0.1.0".into(),
        };
        let certificate =
            sign_certificate(&payload, DEFAULT_DEV_PRIVATE_KEY_HEX).expect("sign certificate");
        let stored = StoredLicense { certificate };
        let json = serde_json::to_string(&stored).expect("serialize");
        let round_trip: StoredLicense = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(round_trip, stored);
    }
}
