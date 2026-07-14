// SPDX-License-Identifier: AGPL-3.0-or-later
// wxpilot - 面向 AI Agent 的微信小程序自动化 CLI
// Copyright (C) 2026  wuliLiuyue
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::{
    collections::HashMap,
    io::{self, Cursor, Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{mpsc, Arc, Mutex},
    thread,
    time::Duration,
};

use rcgen::{
    CertificateParams, DistinguishedName, DnType, ExtendedKeyUsagePurpose, IsCa, KeyPair,
    KeyUsagePurpose,
};
use rustls::{
    Certificate, ClientConfig, ClientConnection, OwnedTrustAnchor, PrivateKey, RootCertStore,
    ServerConfig, ServerConnection, StreamOwned,
};
use serde_json::Value;
use wxp_store::{RequestLog, StoreWriter};

#[derive(Clone, Default)]
pub struct MockRegistry {
    inner: Arc<Mutex<HashMap<(String, String), String>>>,
}

impl MockRegistry {
    pub fn upsert(&self, method: &str, url: &str, body: &str) {
        let mut inner = self.inner.lock().expect("mock registry lock poisoned");
        inner.insert(
            (normalize_method(method), normalize_mock_url(url)),
            body.to_string(),
        );
    }

    pub fn remove(&self, url: &str) {
        let mut inner = self.inner.lock().expect("mock registry lock poisoned");
        let normalized = normalize_mock_url(url);
        inner.retain(|(_, candidate), _| candidate != &normalized);
    }

    pub fn match_url(&self, method: &str, url: &str) -> Option<String> {
        let inner = self.inner.lock().expect("mock registry lock poisoned");
        inner
            .get(&(normalize_method(method), normalize_mock_url(url)))
            .cloned()
    }
}

#[derive(Clone)]
struct ProxyRuntime {
    store: StoreWriter,
    mocks: MockRegistry,
}

#[derive(Clone)]
struct HttpsMitmConfig {
    cert_path: PathBuf,
    key_path: PathBuf,
}

struct RunningProxy {
    port: u16,
    stop_tx: mpsc::Sender<()>,
    handle: thread::JoinHandle<()>,
}

#[derive(Default)]
pub struct LocalProxy {
    runtime: Option<ProxyRuntime>,
    https: Option<HttpsMitmConfig>,
    running: Option<RunningProxy>,
}

impl LocalProxy {
    pub fn attach_runtime(&mut self, store: StoreWriter, mocks: MockRegistry) {
        self.runtime = Some(ProxyRuntime { store, mocks });
    }

    pub fn configure_https(&mut self, cert_path: &Path, key_path: &Path) -> io::Result<()> {
        if !cert_path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("ca cert missing: {}", cert_path.display()),
            ));
        }
        if !key_path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("ca key missing: {}", key_path.display()),
            ));
        }
        self.https = Some(HttpsMitmConfig {
            cert_path: cert_path.to_path_buf(),
            key_path: key_path.to_path_buf(),
        });
        Ok(())
    }

    pub fn disable_https(&mut self) {
        self.https = None;
    }

    pub fn start(&mut self, port: u16) -> io::Result<u16> {
        if let Some(running) = &self.running {
            return Ok(running.port);
        }

        let listener = TcpListener::bind(("127.0.0.1", port))?;
        listener.set_nonblocking(true)?;
        let bound_port = listener.local_addr()?.port();
        let runtime = self.runtime.clone();
        let https = self.https.clone();
        let (stop_tx, stop_rx) = mpsc::channel::<()>();

        let handle = thread::spawn(move || loop {
            if stop_rx.try_recv().is_ok() {
                break;
            }

            match listener.accept() {
                Ok((stream, _)) => {
                    let runtime = runtime.clone();
                    let https = https.clone();
                    thread::spawn(move || {
                        let _ = handle_client(stream, runtime, https);
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(25));
                }
                Err(_) => break,
            }
        });

        self.running = Some(RunningProxy {
            port: bound_port,
            stop_tx,
            handle,
        });
        Ok(bound_port)
    }

    pub fn stop(&mut self) -> io::Result<()> {
        if let Some(running) = self.running.take() {
            let _ = running.stop_tx.send(());
            let _ = running.handle.join();
        }
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.running.is_some()
    }

    pub fn port(&self) -> Option<u16> {
        self.running.as_ref().map(|running| running.port)
    }
}

impl Drop for LocalProxy {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn handle_client(
    mut client: TcpStream,
    runtime: Option<ProxyRuntime>,
    https: Option<HttpsMitmConfig>,
) -> io::Result<()> {
    client.set_nonblocking(false)?;
    let request_bytes = read_http_message(&mut client)?;
    let request = parse_http_request(&request_bytes)?;

    if request.method.eq_ignore_ascii_case("CONNECT") {
        return handle_connect(client, request, runtime, https);
    }

    handle_http(client, request, runtime)
}

fn handle_connect(
    mut client: TcpStream,
    request: HttpRequest,
    runtime: Option<ProxyRuntime>,
    https: Option<HttpsMitmConfig>,
) -> io::Result<()> {
    let authority = request.target.trim();
    let (host, port) = split_host_port(authority, 443)?;
    client.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")?;
    client.flush()?;

    if let Some(https) = https {
        return handle_https_mitm(client, &host, port, runtime, &https);
    }

    relay_connect_tunnel(client, &host, port)
}

fn handle_http(
    mut client: TcpStream,
    request: HttpRequest,
    runtime: Option<ProxyRuntime>,
) -> io::Result<()> {
    let target = parse_target("http", &request.target, request.header("host").as_deref())?;
    let request_body = bytes_to_text(&request.body);

    if let Some(mock_body) = match_mock(&runtime, &request.method, &target.url) {
        let (response_bytes, response_body, status) = build_mock_response(&mock_body)?;
        client.write_all(&response_bytes)?;
        client.flush()?;
        record_request(
            &runtime,
            &request.method,
            &target.url,
            request_body,
            response_body,
            Some(status),
            true,
        );
        return Ok(());
    }

    let mut upstream = TcpStream::connect((target.host.as_str(), target.port))?;
    let outbound = build_outbound_request(&request, &target, false);
    upstream.write_all(&outbound)?;
    upstream.flush()?;

    let response_bytes = read_http_message(&mut upstream)?;
    let response = parse_http_response(&response_bytes)?;
    client.write_all(&response_bytes)?;
    client.flush()?;

    record_request(
        &runtime,
        &request.method,
        &target.url,
        request_body,
        bytes_to_text(&response.body),
        Some(response.status),
        false,
    );
    Ok(())
}

fn handle_https_mitm(
    client: TcpStream,
    host: &str,
    port: u16,
    runtime: Option<ProxyRuntime>,
    https: &HttpsMitmConfig,
) -> io::Result<()> {
    let server_config = Arc::new(build_mitm_server_config(https, host)?);
    let server_connection = ServerConnection::new(server_config).map_err(io::Error::other)?;
    let mut client_tls = StreamOwned::new(server_connection, client);

    let request_bytes = read_http_message(&mut client_tls)?;
    let request = parse_http_request(&request_bytes)?;
    let target = parse_target("https", &request.target, request.header("host").as_deref())?;
    let request_body = bytes_to_text(&request.body);

    if let Some(mock_body) = match_mock(&runtime, &request.method, &target.url) {
        let (response_bytes, response_body, status) = build_mock_response(&mock_body)?;
        client_tls.write_all(&response_bytes)?;
        client_tls.flush()?;
        record_request(
            &runtime,
            &request.method,
            &target.url,
            request_body,
            response_body,
            Some(status),
            true,
        );
        return Ok(());
    }

    let client_config = Arc::new(build_upstream_client_config(https)?);
    let server_name = host.try_into().map_err(io::Error::other)?;
    let upstream_tcp = TcpStream::connect((host, port))?;
    let upstream_connection =
        ClientConnection::new(client_config, server_name).map_err(io::Error::other)?;
    let mut upstream_tls = StreamOwned::new(upstream_connection, upstream_tcp);

    let outbound = build_outbound_request(&request, &target, true);
    upstream_tls.write_all(&outbound)?;
    upstream_tls.flush()?;

    let response_bytes = read_http_message(&mut upstream_tls)?;
    let response = parse_http_response(&response_bytes)?;
    client_tls.write_all(&response_bytes)?;
    client_tls.flush()?;

    record_request(
        &runtime,
        &request.method,
        &target.url,
        request_body,
        bytes_to_text(&response.body),
        Some(response.status),
        false,
    );
    Ok(())
}

fn relay_connect_tunnel(mut client: TcpStream, host: &str, port: u16) -> io::Result<()> {
    let mut upstream = TcpStream::connect((host, port))?;
    let mut client_reader = client.try_clone()?;
    let mut upstream_writer = upstream.try_clone()?;
    let join = thread::spawn(move || {
        let _ = io::copy(&mut client_reader, &mut upstream_writer);
        let _ = upstream_writer.shutdown(Shutdown::Write);
    });

    let _ = io::copy(&mut upstream, &mut client);
    let _ = client.shutdown(Shutdown::Write);
    let _ = join.join();
    Ok(())
}

fn match_mock(runtime: &Option<ProxyRuntime>, method: &str, url: &str) -> Option<String> {
    runtime
        .as_ref()
        .and_then(|runtime| runtime.mocks.match_url(method, url))
}

fn record_request(
    runtime: &Option<ProxyRuntime>,
    method: &str,
    url: &str,
    req_body: Option<String>,
    res_body: Option<String>,
    status: Option<u16>,
    mocked: bool,
) {
    if let Some(runtime) = runtime {
        let _ = runtime.store.insert_request(RequestLog::with_meta(
            method, url, req_body, res_body, status, mocked,
        ));
    }
}

fn normalize_mock_url(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("http://") {
        return normalize_absolute_mock_url("http", rest);
    }
    if let Some(rest) = url.strip_prefix("https://") {
        return normalize_absolute_mock_url("https", rest);
    }
    url.to_string()
}

fn normalize_absolute_mock_url(scheme: &str, rest: &str) -> String {
    let (authority, path) = match rest.find('/') {
        Some(index) => (&rest[..index], &rest[index..]),
        None => (rest, "/"),
    };
    let default_port = default_port_for_scheme(scheme);
    let Ok((host, port)) = split_host_port(authority, default_port) else {
        return format!("{scheme}://{rest}");
    };

    if port == default_port {
        format!("{scheme}://{host}{path}")
    } else {
        format!("{scheme}://{host}:{port}{path}")
    }
}

fn build_mitm_server_config(https: &HttpsMitmConfig, host: &str) -> io::Result<ServerConfig> {
    let ca_cert_pem = std::fs::read_to_string(&https.cert_path)?;
    let ca_key_pem = std::fs::read_to_string(&https.key_path)?;
    let ca_params = CertificateParams::from_ca_cert_pem(&ca_cert_pem).map_err(io::Error::other)?;
    let ca_key = KeyPair::from_pem(&ca_key_pem).map_err(io::Error::other)?;
    let ca_cert = ca_params.self_signed(&ca_key).map_err(io::Error::other)?;

    let mut params = CertificateParams::new(vec![host.to_string()]).map_err(io::Error::other)?;
    let mut distinguished_name = DistinguishedName::new();
    distinguished_name.push(DnType::CommonName, host);
    params.distinguished_name = distinguished_name;
    params.is_ca = IsCa::NoCa;
    params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyEncipherment,
    ];
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];

    let key_pair = KeyPair::generate().map_err(io::Error::other)?;
    let cert = params
        .signed_by(&key_pair, &ca_cert, &ca_key)
        .map_err(io::Error::other)?;

    ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(
            vec![Certificate(cert.der().to_vec())],
            PrivateKey(key_pair.serialize_der()),
        )
        .map_err(io::Error::other)
}

fn build_upstream_client_config(https: &HttpsMitmConfig) -> io::Result<ClientConfig> {
    let ca_cert_pem = std::fs::read_to_string(&https.cert_path)?;
    let mut root_store = RootCertStore::empty();
    root_store.add_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.iter().map(|anchor| {
        OwnedTrustAnchor::from_subject_spki_name_constraints(
            anchor.subject,
            anchor.spki,
            anchor.name_constraints,
        )
    }));

    let ca_certs = rustls_pemfile::certs(&mut Cursor::new(ca_cert_pem.as_bytes()))
        .map_err(io::Error::other)?;
    root_store.add_parsable_certificates(&ca_certs);

    Ok(ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth())
}

fn build_mock_response(payload: &str) -> io::Result<(Vec<u8>, Option<String>, u16)> {
    let value: Value = serde_json::from_str(payload).map_err(io::Error::other)?;
    let status = value.get("status").and_then(Value::as_u64).unwrap_or(200);
    let body_value = value.get("body").cloned().unwrap_or(Value::Null);
    let body = match body_value {
        Value::Null => String::new(),
        Value::String(text) => text,
        other => other.to_string(),
    };

    let mut response = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n",
        status,
        reason_phrase_for_status(status as u16),
        body.len()
    );

    if matches!(
        value.get("body"),
        Some(Value::Object(_) | Value::Array(_) | Value::Bool(_) | Value::Number(_))
    ) {
        response.push_str("Content-Type: application/json\r\n");
    }

    if let Some(headers) = value.get("headers").and_then(Value::as_object) {
        for (name, header_value) in headers {
            if let Some(text) = header_value.as_str() {
                response.push_str(name);
                response.push_str(": ");
                response.push_str(text);
                response.push_str("\r\n");
            }
        }
    }

    response.push_str("\r\n");
    let mut response_bytes = response.into_bytes();
    response_bytes.extend_from_slice(body.as_bytes());

    let response_body = if body.is_empty() { None } else { Some(body) };
    Ok((response_bytes, response_body, status as u16))
}

fn read_http_message(stream: &mut impl Read) -> io::Result<Vec<u8>> {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];
    let mut headers_end = None;

    while headers_end.is_none() {
        let size = stream.read(&mut chunk)?;
        if size == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..size]);
        headers_end = buffer.windows(4).position(|window| window == b"\r\n\r\n");
    }

    let headers_end = headers_end
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "http_headers_incomplete"))?
        + 4;
    let headers = String::from_utf8_lossy(&buffer[..headers_end]);
    if is_chunked_transfer_encoding(&headers) {
        read_chunked_message(stream, &mut buffer, headers_end)?;
        return Ok(buffer);
    }

    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.eq_ignore_ascii_case("Content-Length") {
                value.trim().parse::<usize>().ok()
            } else {
                None
            }
        })
        .unwrap_or(0);

    while buffer.len() < headers_end + content_length {
        let size = stream.read(&mut chunk)?;
        if size == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..size]);
    }

    Ok(buffer)
}

#[derive(Debug, Clone)]
struct HttpRequest {
    method: String,
    target: String,
    version: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl HttpRequest {
    fn header(&self, name: &str) -> Option<String> {
        self.headers
            .iter()
            .find(|(header, _)| header.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.clone())
    }
}

#[derive(Debug, Clone)]
struct HttpResponse {
    status: u16,
    body: Vec<u8>,
}

#[derive(Debug, Clone)]
struct Target {
    host: String,
    port: u16,
    path: String,
    url: String,
}

fn parse_http_request(bytes: &[u8]) -> io::Result<HttpRequest> {
    let headers_end = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "http_headers_missing"))?;
    let head = String::from_utf8(bytes[..headers_end].to_vec())
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let mut lines = head.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "request_line_missing"))?;
    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "method_missing"))?
        .to_string();
    let target = parts
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "target_missing"))?
        .to_string();
    let version = parts
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "version_missing"))?
        .to_string();

    let headers = lines
        .filter(|line| !line.is_empty())
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.to_string(), value.trim().to_string()))
        .collect();

    Ok(HttpRequest {
        method,
        target,
        version,
        headers,
        body: bytes[headers_end + 4..].to_vec(),
    })
}

fn parse_http_response(bytes: &[u8]) -> io::Result<HttpResponse> {
    let headers_end = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "http_headers_missing"))?;
    let head = String::from_utf8(bytes[..headers_end].to_vec())
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let status_line = head
        .split("\r\n")
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "status_line_missing"))?;
    let status = status_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "status_missing"))?
        .parse::<u16>()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let headers = String::from_utf8_lossy(&bytes[..headers_end]);
    let body = if is_chunked_transfer_encoding(&headers) {
        decode_chunked_body(&bytes[headers_end + 4..])?
    } else {
        bytes[headers_end + 4..].to_vec()
    };
    Ok(HttpResponse { status, body })
}

fn parse_target(scheme: &str, target: &str, host_header: Option<&str>) -> io::Result<Target> {
    if let Some(rest) = target.strip_prefix("http://") {
        return parse_absolute_target("http", rest);
    }
    if let Some(rest) = target.strip_prefix("https://") {
        return parse_absolute_target("https", rest);
    }

    let authority = host_header.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "host_header_missing_for_origin_form",
        )
    })?;
    let (host, port) = split_host_port(authority, default_port_for_scheme(scheme))?;
    let path = if target.is_empty() {
        "/".to_string()
    } else {
        target.to_string()
    };
    Ok(Target {
        host: host.clone(),
        port,
        url: format!("{scheme}://{host}:{port}{path}"),
        path,
    })
}

fn parse_absolute_target(scheme: &str, rest: &str) -> io::Result<Target> {
    let (authority, path) = match rest.find('/') {
        Some(index) => (&rest[..index], &rest[index..]),
        None => (rest, "/"),
    };
    let (host, port) = split_host_port(authority, default_port_for_scheme(scheme))?;
    Ok(Target {
        host: host.clone(),
        port,
        path: path.to_string(),
        url: format!("{scheme}://{host}:{port}{path}"),
    })
}

fn build_outbound_request(request: &HttpRequest, target: &Target, force_close: bool) -> Vec<u8> {
    let mut message = format!("{} {} {}\r\n", request.method, target.path, request.version);
    let mut has_host = false;

    for (name, value) in &request.headers {
        if name.eq_ignore_ascii_case("proxy-connection") {
            continue;
        }
        if name.eq_ignore_ascii_case("connection") && force_close {
            continue;
        }
        if name.eq_ignore_ascii_case("host") {
            has_host = true;
        }
        message.push_str(name);
        message.push_str(": ");
        message.push_str(value);
        message.push_str("\r\n");
    }

    if !has_host {
        message.push_str("Host: ");
        message.push_str(&format!("{}:{}", target.host, target.port));
        message.push_str("\r\n");
    }
    if force_close {
        message.push_str("Connection: close\r\n");
    }
    message.push_str("\r\n");

    let mut bytes = message.into_bytes();
    bytes.extend_from_slice(&request.body);
    bytes
}

fn split_host_port(authority: &str, default_port: u16) -> io::Result<(String, u16)> {
    if let Some(host) = authority.strip_prefix('[') {
        let (host, rest) = host
            .split_once(']')
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid_ipv6_authority"))?;
        let port = rest
            .strip_prefix(':')
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(default_port);
        return Ok((host.to_string(), port));
    }

    match authority.rsplit_once(':') {
        Some((host, port)) if !host.contains(':') => {
            let port = port.parse::<u16>().map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("invalid_port: {error}"),
                )
            })?;
            Ok((host.to_string(), port))
        }
        _ => Ok((authority.to_string(), default_port)),
    }
}

fn default_port_for_scheme(scheme: &str) -> u16 {
    if scheme.eq_ignore_ascii_case("https") {
        443
    } else {
        80
    }
}

fn bytes_to_text(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        None
    } else {
        Some(String::from_utf8_lossy(bytes).to_string())
    }
}

fn is_chunked_transfer_encoding(headers: &str) -> bool {
    headers.lines().any(|line| {
        let Some((name, value)) = line.split_once(':') else {
            return false;
        };
        name.eq_ignore_ascii_case("Transfer-Encoding")
            && value
                .split(',')
                .any(|item| item.trim().eq_ignore_ascii_case("chunked"))
    })
}

fn read_chunked_message(
    stream: &mut impl Read,
    buffer: &mut Vec<u8>,
    headers_end: usize,
) -> io::Result<()> {
    let mut cursor = headers_end;
    let mut chunk = [0_u8; 1024];

    loop {
        let line_end = loop {
            if let Some(relative_end) = find_bytes(&buffer[cursor..], b"\r\n") {
                break cursor + relative_end;
            }
            let size = stream.read(&mut chunk)?;
            if size == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "chunk_size_incomplete",
                ));
            }
            buffer.extend_from_slice(&chunk[..size]);
        };

        let chunk_size = std::str::from_utf8(&buffer[cursor..line_end])
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?
            .split(';')
            .next()
            .unwrap_or_default()
            .trim();
        let chunk_size = usize::from_str_radix(chunk_size, 16).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid_chunk_size: {error}"),
            )
        })?;
        if chunk_size == 0 {
            while !buffer.ends_with(b"\r\n\r\n") {
                let size = stream.read(&mut chunk)?;
                if size == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "chunk_trailers_incomplete",
                    ));
                }
                buffer.extend_from_slice(&chunk[..size]);
            }
            break;
        }

        let chunk_end = line_end + 2 + chunk_size + 2;
        while buffer.len() < chunk_end {
            let size = stream.read(&mut chunk)?;
            if size == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "chunk_body_incomplete",
                ));
            }
            buffer.extend_from_slice(&chunk[..size]);
        }

        cursor = chunk_end;
    }

    Ok(())
}

fn decode_chunked_body(bytes: &[u8]) -> io::Result<Vec<u8>> {
    let mut decoded = Vec::new();
    let mut cursor = 0;

    loop {
        let line_end = find_bytes(&bytes[cursor..], b"\r\n")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "chunk_size_missing"))?
            + cursor;
        let chunk_size = std::str::from_utf8(&bytes[cursor..line_end])
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?
            .split(';')
            .next()
            .unwrap_or_default()
            .trim();
        let chunk_size = usize::from_str_radix(chunk_size, 16).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid_chunk_size: {error}"),
            )
        })?;
        let chunk_start = line_end + 2;
        let chunk_end = chunk_start + chunk_size;
        if chunk_end + 2 > bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "chunk_body_truncated",
            ));
        }

        if chunk_size == 0 {
            return Ok(decoded);
        }

        decoded.extend_from_slice(&bytes[chunk_start..chunk_end]);
        cursor = chunk_end + 2;
    }
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn reason_phrase_for_status(status: u16) -> &'static str {
    match status {
        100 => "Continue",
        101 => "Switching Protocols",
        102 => "Processing",
        103 => "Early Hints",
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        203 => "Non-Authoritative Information",
        204 => "No Content",
        205 => "Reset Content",
        206 => "Partial Content",
        300 => "Multiple Choices",
        301 => "Moved Permanently",
        302 => "Found",
        303 => "See Other",
        304 => "Not Modified",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        409 => "Conflict",
        410 => "Gone",
        413 => "Content Too Large",
        415 => "Unsupported Media Type",
        418 => "I'm a teapot",
        422 => "Unprocessable Content",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "",
    }
}

fn normalize_method(method: &str) -> String {
    method.to_ascii_uppercase()
}
