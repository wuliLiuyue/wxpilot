use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(debug_assertions)]
use wxp_license::test_keys::PUBLIC_KEY_HEX as DEBUG_EMBEDDED_PUBLIC_KEY_HEX;
use wxp_license::{
    verify_certificate, ActivationErrorResponse, ActivationRequest, ActivationResponse,
    SignedLicenseCertificate,
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
    verify_certificate(
        &stored.certificate,
        embedded_public_key_hex().map_err(CliError::LicenseInvalid)?,
    )
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
    let server_url = license_server_url().map_err(CliError::ActivationFailed)?;
    let client =
        ureq::post(&format!("{server_url}/v1/activate")).set("Content-Type", "application/json");
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

    verify_certificate(
        &certificate,
        embedded_public_key_hex().map_err(CliError::ActivationFailed)?,
    )
    .map_err(|error| CliError::ActivationFailed(format!("服务端返回的证书校验失败：{error}")))?;
    if certificate.payload.machine_fingerprint != machine_fingerprint {
        return Err(CliError::ActivationFailed(
            "服务端返回的证书与当前机器不匹配".into(),
        ));
    }

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

fn license_server_url() -> Result<String, String> {
    #[cfg(debug_assertions)]
    {
        return Ok(env::var("WXP_LICENSE_SERVER_URL")
            .ok()
            .or_else(|| option_env!("WXP_LICENSE_SERVER_URL").map(ToOwned::to_owned))
            .unwrap_or_else(|| "http://127.0.0.1:8787".into()));
    }

    #[cfg(not(debug_assertions))]
    {
        option_env!("WXP_LICENSE_SERVER_URL")
            .map(ToOwned::to_owned)
            .ok_or_else(|| "当前构建缺少内嵌激活服务地址".into())
    }
}

fn embedded_public_key_hex() -> Result<&'static str, String> {
    #[cfg(debug_assertions)]
    {
        return Ok(DEBUG_EMBEDDED_PUBLIC_KEY_HEX);
    }

    #[cfg(not(debug_assertions))]
    {
        option_env!("WXP_LICENSE_EMBEDDED_PUBLIC_KEY_HEX")
            .ok_or_else(|| "当前构建缺少内嵌授权公钥".into())
    }
}

fn runtime_dir() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".wxpilot")
}

fn machine_fingerprint() -> Result<String, CliError> {
    #[cfg(debug_assertions)]
    if let Some(value) = env::var_os("WXP_LICENSE_TEST_MACHINE_ID") {
        return Ok(hash_machine_identity(&value.to_string_lossy()));
    }

    let identity = platform_machine_identity().ok_or_else(|| {
        CliError::Message("无法读取机器唯一标识，当前平台不能执行离线授权校验".into())
    })?;
    Ok(hash_machine_identity(&identity))
}

fn hash_machine_identity(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(target_os = "macos")]
fn platform_machine_identity() -> Option<String> {
    let output = Command::new("ioreg")
        .args(["-rd1", "-c", "IOPlatformExpertDevice"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    extract_quoted_value(&stdout, "\"IOPlatformUUID\" = \"")
}

#[cfg(target_os = "linux")]
fn platform_machine_identity() -> Option<String> {
    for path in ["/etc/machine-id", "/var/lib/dbus/machine-id"] {
        let value = fs::read_to_string(path).ok()?;
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn platform_machine_identity() -> Option<String> {
    let output = Command::new("wmic")
        .args(["csproduct", "get", "UUID"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    stdout
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && *line != "UUID")
        .map(ToOwned::to_owned)
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn platform_machine_identity() -> Option<String> {
    None
}

#[cfg(target_os = "macos")]
fn extract_quoted_value(raw: &str, prefix: &str) -> Option<String> {
    raw.lines().find_map(|line| {
        let start = line.find(prefix)? + prefix.len();
        let rest = &line[start..];
        let end = rest.find('"')?;
        Some(rest[..end].to_string())
    })
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::sync::{Mutex, OnceLock};

    use super::{hash_machine_identity, license_server_url, restore_env, StoredLicense};
    use wxp_license::{sign_certificate, test_keys::PRIVATE_KEY_HEX, LicenseCertificate};

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
        let certificate = sign_certificate(&payload, PRIVATE_KEY_HEX).expect("sign certificate");
        let stored = StoredLicense { certificate };
        let json = serde_json::to_string(&stored).expect("serialize");
        let round_trip: StoredLicense = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(round_trip, stored);
    }

    #[test]
    fn debug_license_server_url_defaults_to_localhost() {
        let _guard = env_lock().lock().expect("lock env");
        let previous = env::var_os("WXP_LICENSE_SERVER_URL");
        env::remove_var("WXP_LICENSE_SERVER_URL");

        let url = license_server_url().expect("server url");

        restore_env("WXP_LICENSE_SERVER_URL", previous);
        assert_eq!(url, "http://127.0.0.1:8787");
    }

    #[test]
    fn debug_license_server_url_accepts_runtime_override() {
        let _guard = env_lock().lock().expect("lock env");
        let previous = env::var_os("WXP_LICENSE_SERVER_URL");
        env::set_var("WXP_LICENSE_SERVER_URL", "http://127.0.0.1:9999");

        let url = license_server_url().expect("server url");

        restore_env("WXP_LICENSE_SERVER_URL", previous);
        assert_eq!(url, "http://127.0.0.1:9999");
    }

    fn env_lock() -> &'static Mutex<()> {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        ENV_LOCK.get_or_init(|| Mutex::new(()))
    }
}

#[cfg(test)]
fn restore_env(key: &str, value: Option<std::ffi::OsString>) {
    match value {
        Some(value) => env::set_var(key, value),
        None => env::remove_var(key),
    }
}
