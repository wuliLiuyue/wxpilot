use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Utc;
use rusqlite::{
    params, Connection, Error as SqlError, ErrorCode, OptionalExtension, TransactionBehavior,
};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};
use wxp_license::{
    hash_license_key, sign_certificate, ActivationErrorResponse, ActivationRequest,
    ActivationResponse, LicenseCertificate, SignedLicenseCertificate,
};

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_addr: String,
    pub database_path: PathBuf,
    pub private_key_hex: String,
    pub certificate_key_id: String,
}

impl ServerConfig {
    pub fn from_env() -> Result<Self, ServerError> {
        let bind_addr =
            std::env::var("WXP_LICENSE_SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:8787".into());
        let database_path = std::env::var_os("WXP_LICENSE_DB")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(".wxpilot-license/license.db"));
        let private_key_hex = std::env::var("WXP_LICENSE_PRIVATE_KEY_HEX")
            .map_err(|_| ServerError::MissingConfig("WXP_LICENSE_PRIVATE_KEY_HEX".into()))?;
        let certificate_key_id =
            std::env::var("WXP_LICENSE_KEY_ID").unwrap_or_else(|_| "dev".into());

        Ok(Self {
            bind_addr,
            database_path,
            private_key_hex,
            certificate_key_id,
        })
    }
}

#[derive(Debug)]
pub enum ServerError {
    Database(String),
    Http(String),
    InvalidRequest(String),
    MissingConfig(String),
    InvalidLicenseKey,
    InactiveLicense,
    AlreadyBound,
    Signing(String),
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(message) => write!(f, "database error: {message}"),
            Self::Http(message) => write!(f, "http error: {message}"),
            Self::InvalidRequest(message) => write!(f, "invalid request: {message}"),
            Self::MissingConfig(name) => write!(f, "missing required config: {name}"),
            Self::InvalidLicenseKey => f.write_str("license key does not exist"),
            Self::InactiveLicense => f.write_str("license is inactive"),
            Self::AlreadyBound => f.write_str("license key already bound to another machine"),
            Self::Signing(message) => write!(f, "signing error: {message}"),
        }
    }
}

impl std::error::Error for ServerError {}

pub fn serve(config: ServerConfig) -> Result<(), ServerError> {
    init_db(&config.database_path)?;
    let server =
        Server::http(&config.bind_addr).map_err(|error| ServerError::Http(error.to_string()))?;
    for request in server.incoming_requests() {
        if let Err(error) = handle_request(&config, request) {
            eprintln!("license-server error: {error}");
        }
    }
    Ok(())
}

pub fn init_db(path: &Path) -> Result<(), ServerError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| ServerError::Database(error.to_string()))?;
    }

    let connection =
        Connection::open(path).map_err(|error| ServerError::Database(error.to_string()))?;
    connection
        .execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS licenses (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                key_hash TEXT NOT NULL UNIQUE,
                status TEXT NOT NULL DEFAULT 'active',
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS activations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                license_id INTEGER NOT NULL UNIQUE,
                machine_fingerprint TEXT NOT NULL,
                activated_at TEXT NOT NULL,
                certificate_key_id TEXT NOT NULL,
                FOREIGN KEY (license_id) REFERENCES licenses(id)
            );
            "#,
        )
        .map_err(|error| ServerError::Database(error.to_string()))?;
    Ok(())
}

pub fn seed_license(path: &Path, license_key: &str) -> Result<(), ServerError> {
    init_db(path)?;
    let connection =
        Connection::open(path).map_err(|error| ServerError::Database(error.to_string()))?;
    connection
        .execute(
            "INSERT OR IGNORE INTO licenses (key_hash, status, created_at) VALUES (?1, 'active', ?2)",
            params![hash_license_key(license_key), Utc::now().to_rfc3339()],
        )
        .map_err(|error| ServerError::Database(error.to_string()))?;
    Ok(())
}

pub fn activate(
    config: &ServerConfig,
    request: ActivationRequest,
) -> Result<SignedLicenseCertificate, ServerError> {
    if request.license_key.trim().is_empty() {
        return Err(ServerError::InvalidRequest("licenseKey is required".into()));
    }
    if request.machine_fingerprint.trim().is_empty() {
        return Err(ServerError::InvalidRequest(
            "machineFingerprint is required".into(),
        ));
    }

    init_db(&config.database_path)?;
    let mut connection = Connection::open(&config.database_path)
        .map_err(|error| ServerError::Database(error.to_string()))?;
    connection
        .busy_timeout(Duration::from_secs(1))
        .map_err(|error| ServerError::Database(error.to_string()))?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| ServerError::Database(error.to_string()))?;
    let key_hash = hash_license_key(&request.license_key);

    let license = transaction
        .query_row(
            "SELECT id, status FROM licenses WHERE key_hash = ?1",
            params![key_hash],
            |row| {
                let id: i64 = row.get(0)?;
                let status: String = row.get(1)?;
                Ok((id, status))
            },
        )
        .optional()
        .map_err(|error| ServerError::Database(error.to_string()))?
        .ok_or(ServerError::InvalidLicenseKey)?;

    if license.1 != "active" {
        return Err(ServerError::InactiveLicense);
    }

    let existing_machine = transaction
        .query_row(
            "SELECT machine_fingerprint FROM activations WHERE license_id = ?1",
            params![license.0],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| ServerError::Database(error.to_string()))?;

    match existing_machine {
        Some(machine) if machine != request.machine_fingerprint => {
            return Err(ServerError::AlreadyBound);
        }
        Some(_) => {}
        None => {
            match transaction.execute(
                    "INSERT INTO activations (license_id, machine_fingerprint, activated_at, certificate_key_id) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        license.0,
                        request.machine_fingerprint,
                        Utc::now().to_rfc3339(),
                        config.certificate_key_id
                    ],
                ) {
                Ok(_) => {}
                Err(error) if is_unique_constraint(&error) => {
                    let rebound_machine = transaction
                        .query_row(
                            "SELECT machine_fingerprint FROM activations WHERE license_id = ?1",
                            params![license.0],
                            |row| row.get::<_, String>(0),
                        )
                        .optional()
                        .map_err(|query_error| ServerError::Database(query_error.to_string()))?;
                    match rebound_machine {
                        Some(machine) if machine != request.machine_fingerprint => {
                            return Err(ServerError::AlreadyBound);
                        }
                        Some(_) => {}
                        None => return Err(ServerError::Database(error.to_string())),
                    }
                }
                Err(error) => return Err(ServerError::Database(error.to_string())),
            }
        }
    }

    transaction
        .commit()
        .map_err(|error| ServerError::Database(error.to_string()))?;

    let payload = LicenseCertificate {
        version: wxp_license::CERTIFICATE_VERSION,
        license_id: format!("lic_{}", license.0),
        key_id: config.certificate_key_id.clone(),
        machine_fingerprint: request.machine_fingerprint,
        issued_at: Utc::now().to_rfc3339(),
        client_version: request.client_version,
    };

    sign_certificate(&payload, &config.private_key_hex)
        .map_err(|error| ServerError::Signing(error.to_string()))
}

fn is_unique_constraint(error: &SqlError) -> bool {
    matches!(
        error,
        SqlError::SqliteFailure(details, _)
            if details.code == ErrorCode::ConstraintViolation
    )
}

fn health_payload(config: &ServerConfig) -> serde_json::Value {
    serde_json::json!({
        "ok": true,
        "databasePath": config.database_path.display().to_string(),
    })
}

fn handle_request(config: &ServerConfig, mut request: Request) -> Result<(), ServerError> {
    match (request.method(), request.url()) {
        (&Method::Get, "/healthz") => respond_json(request, StatusCode(200), &health_payload(config)),
        (&Method::Post, "/v1/activate") => {
            let mut body = String::new();
            request
                .as_reader()
                .read_to_string(&mut body)
                .map_err(|error| ServerError::InvalidRequest(error.to_string()))?;
            let parsed: ActivationRequest = serde_json::from_str(&body)
                .map_err(|error| ServerError::InvalidRequest(error.to_string()))?;
            match activate(config, parsed) {
                Ok(certificate) => respond_json(
                    request,
                    StatusCode(200),
                    &ActivationResponse { certificate },
                ),
                Err(error) => {
                    let (status, body) = error_response(&error);
                    respond_json(request, status, &body)
                }
            }
        }
        _ => respond_json(
            request,
            StatusCode(404),
            &ActivationErrorResponse {
                error: "not_found".into(),
                message: "route not found".into(),
            },
        ),
    }
}

fn error_response(error: &ServerError) -> (StatusCode, ActivationErrorResponse) {
    match error {
        ServerError::InvalidRequest(message) => (
            StatusCode(400),
            ActivationErrorResponse {
                error: "invalid_request".into(),
                message: message.clone(),
            },
        ),
        ServerError::InvalidLicenseKey => (
            StatusCode(404),
            ActivationErrorResponse {
                error: "invalid_license_key".into(),
                message: error.to_string(),
            },
        ),
        ServerError::InactiveLicense => (
            StatusCode(403),
            ActivationErrorResponse {
                error: "inactive_license".into(),
                message: error.to_string(),
            },
        ),
        ServerError::AlreadyBound => (
            StatusCode(409),
            ActivationErrorResponse {
                error: "already_bound".into(),
                message: error.to_string(),
            },
        ),
        ServerError::Database(_)
        | ServerError::Http(_)
        | ServerError::MissingConfig(_)
        | ServerError::Signing(_) => (
            StatusCode(500),
            ActivationErrorResponse {
                error: "internal_error".into(),
                message: error.to_string(),
            },
        ),
    }
}

fn respond_json<T: serde::Serialize>(
    request: Request,
    status: StatusCode,
    body: &T,
) -> Result<(), ServerError> {
    let payload = serde_json::to_vec(body).map_err(|error| ServerError::Http(error.to_string()))?;
    let header = Header::from_bytes("Content-Type", "application/json")
        .map_err(|_| ServerError::Http("invalid header".into()))?;
    request
        .respond(
            Response::from_data(payload)
                .with_status_code(status)
                .with_header(header),
        )
        .map_err(|error| ServerError::Http(error.to_string()))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{health_payload, ServerConfig};

    #[test]
    fn health_payload_reports_ok_and_database_path() {
        let config = ServerConfig {
            bind_addr: "127.0.0.1:8787".into(),
            database_path: PathBuf::from("/tmp/license.db"),
            private_key_hex: "deadbeef".into(),
            certificate_key_id: "dev".into(),
        };

        let payload = health_payload(&config);

        assert_eq!(payload.get("ok").and_then(serde_json::Value::as_bool), Some(true));
        assert_eq!(
            payload.get("databasePath").and_then(serde_json::Value::as_str),
            Some("/tmp/license.db")
        );
    }
}
