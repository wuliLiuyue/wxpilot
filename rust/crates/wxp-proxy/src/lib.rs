use std::{
    collections::HashMap,
    io::{self, Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

#[derive(Default)]
pub struct MockRegistry {
    items: HashMap<String, String>,
}

impl MockRegistry {
    pub fn upsert(&mut self, method: &str, url: &str, body: &str) {
        self.items
            .insert(format!("{method}:{url}"), body.to_string());
    }

    pub fn remove(&mut self, url: &str) {
        self.items.retain(|key, _| !key.ends_with(url));
    }

    pub fn match_url(&self, method: &str, url: &str) -> Option<&String> {
        self.items.get(&format!("{method}:{url}"))
    }
}

#[derive(Default)]
pub struct LocalProxy {
    handle: Option<ProxyHandle>,
}

struct ProxyHandle {
    port: u16,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl LocalProxy {
    pub fn start(&mut self, port: u16) -> io::Result<u16> {
        self.stop()?;

        let listener = TcpListener::bind(("127.0.0.1", port))?;
        listener.set_nonblocking(true)?;
        let bound_port = listener.local_addr()?.port();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_for_thread = stop.clone();
        let thread = thread::spawn(move || accept_loop(listener, stop_for_thread));

        self.handle = Some(ProxyHandle {
            port: bound_port,
            stop,
            thread: Some(thread),
        });

        Ok(bound_port)
    }

    pub fn stop(&mut self) -> io::Result<()> {
        if let Some(mut handle) = self.handle.take() {
            handle.stop.store(true, Ordering::SeqCst);
            let _ = TcpStream::connect(("127.0.0.1", handle.port));
            if let Some(thread) = handle.thread.take() {
                let _ = thread.join();
            }
        }

        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.handle.is_some()
    }

    pub fn port(&self) -> Option<u16> {
        self.handle.as_ref().map(|handle| handle.port)
    }
}

impl Drop for LocalProxy {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn accept_loop(listener: TcpListener, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                thread::spawn(move || {
                    let _ = handle_client(stream);
                });
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(50));
            }
            Err(_) => break,
        }
    }
}

fn handle_client(mut client: TcpStream) -> io::Result<()> {
    client.set_nonblocking(false)?;

    let (header_bytes, remaining) = match read_request_prelude(&mut client) {
        Ok(parts) => parts,
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(()),
        Err(error) => return Err(error),
    };
    let header_text = String::from_utf8_lossy(&header_bytes);
    let mut lines = header_text.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing request line"))?;
    let (method, target, version) = parse_request_line(request_line)?;
    let headers = lines
        .filter(|line| !line.is_empty())
        .filter_map(parse_header_line)
        .collect::<Vec<_>>();

    if method.eq_ignore_ascii_case("CONNECT") {
        return handle_connect(client, target, remaining);
    }

    handle_http(client, method, target, version, &headers, remaining)
}

fn read_request_prelude(stream: &mut TcpStream) -> io::Result<(Vec<u8>, Vec<u8>)> {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];

    loop {
        let size = stream.read(&mut chunk)?;
        if size == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "connection closed before request headers",
            ));
        }
        buffer.extend_from_slice(&chunk[..size]);

        if let Some(index) = find_headers_end(&buffer) {
            let headers = buffer[..index].to_vec();
            let remaining = buffer[index..].to_vec();
            return Ok((headers, remaining));
        }

        if buffer.len() > 64 * 1024 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "proxy request headers too large",
            ));
        }
    }
}

fn find_headers_end(buffer: &[u8]) -> Option<usize> {
    buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
}

fn parse_request_line(line: &str) -> io::Result<(&str, &str, &str)> {
    let mut parts = line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing method"))?;
    let target = parts
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing target"))?;
    let version = parts
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing version"))?;
    Ok((method, target, version))
}

fn parse_header_line(line: &str) -> Option<(String, String)> {
    let (name, value) = line.split_once(':')?;
    Some((name.trim().to_string(), value.trim().to_string()))
}

fn handle_connect(mut client: TcpStream, target: &str, remaining: Vec<u8>) -> io::Result<()> {
    let (host, port) = parse_host_port(target, 443)?;
    let mut upstream = TcpStream::connect((host.as_str(), port))?;
    client.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")?;
    if !remaining.is_empty() {
        upstream.write_all(&remaining)?;
    }
    bridge_streams(client, upstream)
}

fn handle_http(
    client: TcpStream,
    method: &str,
    target: &str,
    version: &str,
    headers: &[(String, String)],
    remaining: Vec<u8>,
) -> io::Result<()> {
    let (host, port, path) = parse_http_target(target, headers)?;
    let mut upstream = TcpStream::connect((host.as_str(), port))?;

    let mut request = format!("{method} {path} {version}\r\n");
    let mut has_host = false;
    for (name, value) in headers {
        if name.eq_ignore_ascii_case("proxy-connection") {
            continue;
        }
        if name.eq_ignore_ascii_case("host") {
            has_host = true;
        }
        request.push_str(name);
        request.push_str(": ");
        request.push_str(value);
        request.push_str("\r\n");
    }
    if !has_host {
        request.push_str(&format!("Host: {host}:{port}\r\n"));
    }
    request.push_str("\r\n");

    upstream.write_all(request.as_bytes())?;
    if !remaining.is_empty() {
        upstream.write_all(&remaining)?;
    }

    bridge_streams(client, upstream)
}

fn parse_http_target(
    target: &str,
    headers: &[(String, String)],
) -> io::Result<(String, u16, String)> {
    if let Some(rest) = target.strip_prefix("http://") {
        let (authority, path) = match rest.find('/') {
            Some(index) => (&rest[..index], &rest[index..]),
            None => (rest, "/"),
        };
        let (host, port) = parse_host_port(authority, 80)?;
        return Ok((host, port, path.to_string()));
    }

    let host_header = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("host"))
        .map(|(_, value)| value.as_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing host header"))?;
    let (host, port) = parse_host_port(host_header, 80)?;
    let path = if target.is_empty() { "/" } else { target };
    Ok((host, port, path.to_string()))
}

fn parse_host_port(authority: &str, default_port: u16) -> io::Result<(String, u16)> {
    if let Some((host, port)) = authority.rsplit_once(':') {
        if let Ok(port) = port.parse::<u16>() {
            return Ok((host.to_string(), port));
        }
    }

    Ok((authority.to_string(), default_port))
}

fn bridge_streams(client: TcpStream, upstream: TcpStream) -> io::Result<()> {
    let mut client_reader = client.try_clone()?;
    let mut client_writer = client;
    let mut upstream_reader = upstream.try_clone()?;
    let mut upstream_writer = upstream;

    let request_thread = thread::spawn(move || {
        let _ = io::copy(&mut client_reader, &mut upstream_writer);
        let _ = upstream_writer.shutdown(Shutdown::Write);
    });

    let response_thread = thread::spawn(move || {
        let _ = io::copy(&mut upstream_reader, &mut client_writer);
        let _ = client_writer.shutdown(Shutdown::Write);
    });

    let _ = request_thread.join();
    let _ = response_thread.join();

    Ok(())
}
