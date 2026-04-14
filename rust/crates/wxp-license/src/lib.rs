use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const CERTIFICATE_VERSION: u32 = 1;

#[cfg(debug_assertions)]
pub mod test_keys {
    pub const PRIVATE_KEY_HEX: &str =
        "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20";
    pub const PUBLIC_KEY_HEX: &str =
        "79b5562e8fe654f94078b112e8a98ba7901f853ae695bed7e0e3910bad049664";
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivationRequest {
    pub license_key: String,
    pub machine_fingerprint: String,
    pub client_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivationResponse {
    pub certificate: SignedLicenseCertificate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivationErrorResponse {
    pub error: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseCertificate {
    pub version: u32,
    pub license_id: String,
    pub key_id: String,
    pub machine_fingerprint: String,
    pub issued_at: String,
    pub client_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedLicenseCertificate {
    pub payload: LicenseCertificate,
    pub signature: String,
}

#[derive(Debug)]
pub enum LicenseError {
    InvalidHex(String),
    InvalidKeyLength(usize),
    InvalidSignature,
    Serialize(String),
}

impl std::fmt::Display for LicenseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidHex(error) => write!(f, "invalid hex: {error}"),
            Self::InvalidKeyLength(length) => write!(f, "invalid key length: {length}"),
            Self::InvalidSignature => f.write_str("invalid signature"),
            Self::Serialize(error) => write!(f, "serialize error: {error}"),
        }
    }
}

impl std::error::Error for LicenseError {}

pub fn hash_license_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn derive_public_key_hex(private_key_hex: &str) -> Result<String, LicenseError> {
    let signing_key = signing_key_from_hex(private_key_hex)?;
    Ok(hex::encode(signing_key.verifying_key().to_bytes()))
}

pub fn sign_certificate(
    payload: &LicenseCertificate,
    private_key_hex: &str,
) -> Result<SignedLicenseCertificate, LicenseError> {
    let signing_key = signing_key_from_hex(private_key_hex)?;
    let message = canonical_certificate_bytes(payload)?;
    let signature = signing_key.sign(&message);
    Ok(SignedLicenseCertificate {
        payload: payload.clone(),
        signature: hex::encode(signature.to_bytes()),
    })
}

pub fn verify_certificate(
    certificate: &SignedLicenseCertificate,
    public_key_hex: &str,
) -> Result<(), LicenseError> {
    let verifying_key = verifying_key_from_hex(public_key_hex)?;
    let signature_bytes = decode_hex_fixed::<64>(&certificate.signature)?;
    let signature = Signature::from_bytes(&signature_bytes);
    let message = canonical_certificate_bytes(&certificate.payload)?;
    verifying_key
        .verify(&message, &signature)
        .map_err(|_| LicenseError::InvalidSignature)
}

fn canonical_certificate_bytes(payload: &LicenseCertificate) -> Result<Vec<u8>, LicenseError> {
    serde_json::to_vec(payload).map_err(|error| LicenseError::Serialize(error.to_string()))
}

fn signing_key_from_hex(private_key_hex: &str) -> Result<SigningKey, LicenseError> {
    let bytes = decode_hex_fixed::<32>(private_key_hex)?;
    Ok(SigningKey::from_bytes(&bytes))
}

fn verifying_key_from_hex(public_key_hex: &str) -> Result<VerifyingKey, LicenseError> {
    let bytes = decode_hex_fixed::<32>(public_key_hex)?;
    VerifyingKey::from_bytes(&bytes).map_err(|error| LicenseError::InvalidHex(error.to_string()))
}

fn decode_hex_fixed<const N: usize>(raw: &str) -> Result<[u8; N], LicenseError> {
    let bytes = hex::decode(raw).map_err(|error| LicenseError::InvalidHex(error.to_string()))?;
    let length = bytes.len();
    bytes
        .try_into()
        .map_err(|_| LicenseError::InvalidKeyLength(length))
}

#[cfg(test)]
mod tests {
    use super::{
        derive_public_key_hex, sign_certificate, verify_certificate, LicenseCertificate,
    };
    use crate::test_keys::{PRIVATE_KEY_HEX, PUBLIC_KEY_HEX};

    #[test]
    fn default_dev_key_pair_matches() {
        let derived = derive_public_key_hex(PRIVATE_KEY_HEX).expect("derive public key");
        assert_eq!(derived, PUBLIC_KEY_HEX);
    }

    #[test]
    fn signed_certificate_verifies() {
        let payload = LicenseCertificate {
            version: 1,
            license_id: "lic_1".into(),
            key_id: "dev".into(),
            machine_fingerprint: "machine-a".into(),
            issued_at: "2026-04-13T00:00:00Z".into(),
            client_version: "0.1.0".into(),
        };

        let certificate = sign_certificate(&payload, PRIVATE_KEY_HEX).expect("sign certificate");
        verify_certificate(&certificate, PUBLIC_KEY_HEX).expect("verify");
    }
}
