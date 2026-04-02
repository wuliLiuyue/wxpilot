use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
};

use wxp_proxy::{LocalProxy, MockRegistry};

#[test]
fn exact_url_mock_can_be_added_and_removed() {
    let mut registry = MockRegistry::default();
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

    let mut proxy = LocalProxy::default();
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
