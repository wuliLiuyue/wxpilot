use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    sync::Arc,
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, DnType, ExtendedKeyUsagePurpose, IsCa,
    KeyPair, KeyUsagePurpose,
};
use rustls::{
    Certificate, ClientConfig, ClientConnection, PrivateKey, RootCertStore, ServerConfig,
    ServerConnection, StreamOwned,
};
use wxp_proxy::{LocalProxy, MockRegistry};
use wxp_store::StoreWriter;

fn unique_temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("wxp-proxy-test-{name}-{nanos}"));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn write_test_ca(name: &str) -> (PathBuf, PathBuf, String, String) {
    let dir = unique_temp_dir(name);
    let cert_path = dir.join("cert.pem");
    let key_path = dir.join("key.pem");

    let mut params = CertificateParams::new(vec!["wxpilot.local".to_string()]).expect("ca params");
    let mut distinguished_name = DistinguishedName::new();
    distinguished_name.push(DnType::CommonName, "wxpilot test ca");
    params.distinguished_name = distinguished_name;
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.key_usages = vec![
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::CrlSign,
    ];

    let key_pair = KeyPair::generate().expect("generate ca key");
    let cert = params.self_signed(&key_pair).expect("self sign ca");
    let cert_pem = cert.pem();
    let key_pem = key_pair.serialize_pem();

    fs::write(&cert_path, &cert_pem).expect("write ca cert");
    fs::write(&key_path, &key_pem).expect("write ca key");

    (cert_path, key_path, cert_pem, key_pem)
}

fn issue_server_cert(ca_cert_pem: &str, ca_key_pem: &str, host: &str) -> (String, String) {
    let ca_params = CertificateParams::from_ca_cert_pem(ca_cert_pem).expect("load ca cert");
    let ca_key = KeyPair::from_pem(ca_key_pem).expect("load ca key");
    let ca_cert = ca_params.self_signed(&ca_key).expect("rebuild ca cert");

    let mut params = CertificateParams::new(vec![host.to_string()]).expect("leaf params");
    let mut distinguished_name = DistinguishedName::new();
    distinguished_name.push(DnType::CommonName, host);
    params.distinguished_name = distinguished_name;
    params.is_ca = IsCa::NoCa;
    params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyEncipherment,
    ];
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];

    let key_pair = KeyPair::generate().expect("generate leaf key");
    let cert = params
        .signed_by(&key_pair, &ca_cert, &ca_key)
        .expect("sign leaf cert");
    (cert.pem(), key_pair.serialize_pem())
}

fn rustls_certs(pem: &str) -> Vec<Certificate> {
    rustls_pemfile::certs(&mut std::io::Cursor::new(pem.as_bytes()))
        .expect("parse certs")
        .into_iter()
        .map(Certificate)
        .collect()
}

fn rustls_key(pem: &str) -> PrivateKey {
    let mut cursor = std::io::Cursor::new(pem.as_bytes());
    loop {
        match rustls_pemfile::read_one(&mut cursor).expect("read pem item") {
            Some(rustls_pemfile::Item::RSAKey(key))
            | Some(rustls_pemfile::Item::PKCS8Key(key))
            | Some(rustls_pemfile::Item::ECKey(key)) => return PrivateKey(key),
            Some(_) => continue,
            None => panic!("private key missing"),
        }
    }
}

fn read_http_message(stream: &mut impl Read) -> Vec<u8> {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];
    let mut headers_end = None;

    while headers_end.is_none() {
        let size = stream.read(&mut chunk).expect("read headers");
        if size == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..size]);
        headers_end = buffer.windows(4).position(|window| window == b"\r\n\r\n");
    }

    let headers_end = headers_end.expect("http headers terminator") + 4;
    let headers = String::from_utf8_lossy(&buffer[..headers_end]);
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
        let size = stream.read(&mut chunk).expect("read body");
        if size == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..size]);
    }

    buffer
}

#[test]
fn exact_url_mock_can_be_added_and_removed() {
    let registry = MockRegistry::default();
    registry.upsert("GET", "https://wxpilot.test/api", "{\"ok\":true}");
    assert!(registry
        .match_url("GET", "https://wxpilot.test/api")
        .is_some());
    registry.remove("https://wxpilot.test/api");
    assert!(registry
        .match_url("GET", "https://wxpilot.test/api")
        .is_none());
}

#[test]
fn http_proxy_forwards_absolute_form_requests() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind target server");
    let addr = listener.local_addr().expect("target addr");

    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept target connection");
        let mut buffer = [0_u8; 1024];
        let _ = stream.read(&mut buffer).expect("read request");
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
            .expect("write response");
    });

    let writer = StoreWriter::memory().expect("memory store");
    let registry = MockRegistry::default();
    let mut proxy = LocalProxy::default();
    proxy.attach_runtime(writer.clone(), registry);
    let port = proxy.start(0).expect("start proxy");

    let mut client = TcpStream::connect(("127.0.0.1", port)).expect("connect to proxy");
    client
        .write_all(
            format!(
                "GET http://127.0.0.1:{}/hello HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\r\n",
                addr.port(),
                addr.port()
            )
            .as_bytes(),
        )
        .expect("write proxy request");

    let mut response = String::new();
    client
        .read_to_string(&mut response)
        .expect("read proxy response");

    assert!(response.contains("200 OK"));
    assert!(response.ends_with("ok"));
    let logs = writer.list_requests(20, true).expect("list requests");
    assert_eq!(logs.len(), 1);
    assert_eq!(
        logs[0].url,
        format!("http://127.0.0.1:{}/hello", addr.port())
    );
    assert_eq!(logs[0].status, Some(200));
    assert!(!logs[0].mocked);
    assert!(logs[0].ts > 0);
    assert_eq!(logs[0].res_body.as_deref(), Some("ok"));

    proxy.stop().expect("stop proxy");
}

#[test]
fn http_proxy_returns_mocked_response_and_records_it() {
    let writer = StoreWriter::memory().expect("memory store");
    let registry = MockRegistry::default();
    let url = "http://127.0.0.1:18081/mock";
    registry.upsert("GET", url, r#"{"status":201,"body":{"ok":true}}"#);

    let mut proxy = LocalProxy::default();
    proxy.attach_runtime(writer.clone(), registry);
    let port = proxy.start(0).expect("start proxy");

    let mut client = TcpStream::connect(("127.0.0.1", port)).expect("connect to proxy");
    client
        .write_all(
            b"GET http://127.0.0.1:18081/mock HTTP/1.1\r\nHost: 127.0.0.1:18081\r\nConnection: close\r\n\r\n",
        )
        .expect("write proxy request");

    let mut response = String::new();
    client
        .read_to_string(&mut response)
        .expect("read proxy response");

    assert!(response.contains("201 OK"));
    assert!(response.ends_with("{\"ok\":true}"));
    let logs = writer.list_requests(20, true).expect("list requests");
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].url, url);
    assert_eq!(logs[0].status, Some(201));
    assert!(logs[0].mocked);
    assert!(logs[0].ts > 0);
    assert_eq!(logs[0].res_body.as_deref(), Some("{\"ok\":true}"));

    proxy.stop().expect("stop proxy");
}

#[test]
fn connect_tunnel_relays_raw_tcp_payloads() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind target tunnel server");
    let addr = listener.local_addr().expect("tunnel addr");

    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept tunnel connection");
        let mut payload = [0_u8; 5];
        stream
            .read_exact(&mut payload)
            .expect("read tunnel payload");
        assert_eq!(&payload, b"hello");
        stream.write_all(b"world").expect("write tunnel response");
    });

    let mut proxy = LocalProxy::default();
    let port = proxy.start(0).expect("start proxy");

    let mut client = TcpStream::connect(("127.0.0.1", port)).expect("connect to proxy");
    client
        .write_all(
            format!(
                "CONNECT 127.0.0.1:{} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n\r\n",
                addr.port(),
                addr.port()
            )
            .as_bytes(),
        )
        .expect("write connect request");

    let mut response_headers = [0_u8; 64];
    let size = client
        .read(&mut response_headers)
        .expect("read connect response");
    assert!(
        String::from_utf8_lossy(&response_headers[..size]).contains("200 Connection Established"),
        "proxy should establish CONNECT tunnel"
    );

    client.write_all(b"hello").expect("write tunnel payload");
    let mut echoed = [0_u8; 5];
    client
        .read_exact(&mut echoed)
        .expect("read tunnel response");
    assert_eq!(&echoed, b"world");

    proxy.stop().expect("stop proxy");
}

#[test]
fn https_proxy_mitm_decrypts_and_logs_requests() {
    let (ca_cert_path, ca_key_path, ca_cert_pem, ca_key_pem) = write_test_ca("https-forward");
    let (server_cert_pem, server_key_pem) =
        issue_server_cert(&ca_cert_pem, &ca_key_pem, "localhost");
    let server_config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(rustls_certs(&server_cert_pem), rustls_key(&server_key_pem))
        .expect("server config");

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind https target");
    let addr = listener.local_addr().expect("https target addr");
    thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept https target");
        let connection = ServerConnection::new(Arc::new(server_config)).expect("tls server");
        let mut tls = StreamOwned::new(connection, stream);
        let request = String::from_utf8(read_http_message(&mut tls)).expect("request text");
        assert!(request.starts_with("POST /secure HTTP/1.1"));
        assert!(request.ends_with("\r\n\r\nhello"));
        tls.write_all(
            b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\nConnection: close\r\n\r\nsecure-ok",
        )
        .expect("write https response");
        tls.flush().expect("flush https response");
    });

    let writer = StoreWriter::memory().expect("memory store");
    let registry = MockRegistry::default();
    let mut proxy = LocalProxy::default();
    proxy.attach_runtime(writer.clone(), registry);
    proxy
        .configure_https(&ca_cert_path, &ca_key_path)
        .expect("configure https");
    let port = proxy.start(0).expect("start proxy");

    let mut client = TcpStream::connect(("127.0.0.1", port)).expect("connect to proxy");
    client
        .write_all(
            format!(
                "CONNECT localhost:{} HTTP/1.1\r\nHost: localhost:{}\r\n\r\n",
                addr.port(),
                addr.port()
            )
            .as_bytes(),
        )
        .expect("write connect request");

    let connect_response = String::from_utf8(read_http_message(&mut client)).expect("connect text");
    assert!(connect_response.contains("200 Connection Established"));

    let mut root_store = RootCertStore::empty();
    root_store.add_parsable_certificates(
        &rustls_pemfile::certs(&mut std::io::Cursor::new(ca_cert_pem.as_bytes()))
            .expect("parse ca cert"),
    );
    let client_config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let connection = ClientConnection::new(
        Arc::new(client_config),
        "localhost".try_into().expect("server name"),
    )
    .expect("tls client");
    let mut tls = StreamOwned::new(connection, client);

    tls.write_all(
        format!(
            "POST /secure HTTP/1.1\r\nHost: localhost:{}\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello",
            addr.port()
        )
        .as_bytes(),
    )
    .expect("write https request");
    tls.flush().expect("flush https request");

    let response = String::from_utf8(read_http_message(&mut tls)).expect("response text");
    assert!(response.contains("200 OK"));
    assert!(response.ends_with("secure-ok"));

    let logs = writer.list_requests(20, true).expect("list requests");
    assert_eq!(logs.len(), 1);
    assert_eq!(
        logs[0].url,
        format!("https://localhost:{}/secure", addr.port())
    );
    assert_eq!(logs[0].status, Some(200));
    assert!(!logs[0].mocked);
    assert!(logs[0].ts > 0);
    assert_eq!(logs[0].req_body.as_deref(), Some("hello"));
    assert_eq!(logs[0].res_body.as_deref(), Some("secure-ok"));

    proxy.stop().expect("stop proxy");
}

#[test]
fn https_proxy_returns_mocked_response_and_records_it() {
    let (ca_cert_path, ca_key_path, ca_cert_pem, _) = write_test_ca("https-mock");
    let writer = StoreWriter::memory().expect("memory store");
    let registry = MockRegistry::default();
    let url = "https://localhost:18443/mock";
    registry.upsert("GET", url, r#"{"status":202,"body":{"ok":true}}"#);

    let mut proxy = LocalProxy::default();
    proxy.attach_runtime(writer.clone(), registry);
    proxy
        .configure_https(&ca_cert_path, &ca_key_path)
        .expect("configure https");
    let port = proxy.start(0).expect("start proxy");

    let mut client = TcpStream::connect(("127.0.0.1", port)).expect("connect to proxy");
    client
        .write_all(b"CONNECT localhost:18443 HTTP/1.1\r\nHost: localhost:18443\r\n\r\n")
        .expect("write connect request");

    let connect_response = String::from_utf8(read_http_message(&mut client)).expect("connect text");
    assert!(connect_response.contains("200 Connection Established"));

    let mut root_store = RootCertStore::empty();
    root_store.add_parsable_certificates(
        &rustls_pemfile::certs(&mut std::io::Cursor::new(ca_cert_pem.as_bytes()))
            .expect("parse ca cert"),
    );
    let client_config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let connection = ClientConnection::new(
        Arc::new(client_config),
        "localhost".try_into().expect("server name"),
    )
    .expect("tls client");
    let mut tls = StreamOwned::new(connection, client);

    tls.write_all(b"GET /mock HTTP/1.1\r\nHost: localhost:18443\r\nConnection: close\r\n\r\n")
        .expect("write https mock request");
    tls.flush().expect("flush https mock request");

    let response = String::from_utf8(read_http_message(&mut tls)).expect("response text");
    assert!(response.contains("202 OK"));
    assert!(response.ends_with("{\"ok\":true}"));

    let logs = writer.list_requests(20, true).expect("list requests");
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].url, url);
    assert_eq!(logs[0].status, Some(202));
    assert!(logs[0].mocked);
    assert!(logs[0].ts > 0);
    assert_eq!(logs[0].res_body.as_deref(), Some("{\"ok\":true}"));

    proxy.stop().expect("stop proxy");
}
