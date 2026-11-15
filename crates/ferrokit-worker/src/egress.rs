//! The egress proxy: every request Chromium makes (from any tab, frame, worker or WebSocket) goes
//! through here. It resolves each destination itself and refuses this machine's own addresses
//! (loopback, unspecified, link-local, in any written form, including names that resolve to them),
//! so a page being printed can't talk to the worker's internal services such as LibreOffice's control
//! port. The one exception is the job-file server Chromium loads uploaded pages from. Other network
//! addresses, including the home LAN, are allowed: printing LAN pages is intended.
//!
//! It speaks just enough HTTP/1.1 proxy: `CONNECT host:port` (HTTPS, wss) and absolute-form requests
//! for plain HTTP, one request per connection (`Connection: close`), so every request is checked.

use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const MAX_HEAD: usize = 64 * 1024;
const FORBIDDEN: &[u8] = b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";

/// Whether an address belongs to this machine itself (or is link-local, like cloud metadata services).
pub fn blocked(ip: IpAddr) -> bool {
    // IPv4 written as IPv6 (::ffff:a.b.c.d, or the old ::a.b.c.d) is judged as the IPv4 address.
    let ip = match ip {
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => IpAddr::V4(v4),
            None => {
                let s = v6.segments();
                if s[..6] == [0; 6] && !v6.is_loopback() && !v6.is_unspecified() {
                    IpAddr::V4(Ipv4Addr::new((s[6] >> 8) as u8, s[6] as u8, (s[7] >> 8) as u8, s[7] as u8))
                } else {
                    IpAddr::V6(v6)
                }
            }
        },
        v4 => v4,
    };
    match ip {
        IpAddr::V4(v4) => v4.is_loopback() || v4.octets()[0] == 0 || v4.is_link_local(),
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified() || (v6.segments()[0] & 0xffc0) == 0xfe80,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub method: String,
    pub host: String,
    pub port: u16,
    /// Origin-form path for plain HTTP; `None` for CONNECT.
    pub path: Option<String>,
}

/// `host[:port]` with an optional `[v6]` literal.
fn host_port(s: &str, default: u16) -> Option<(String, u16)> {
    if let Some(rest) = s.strip_prefix('[') {
        let (host, after) = rest.split_once(']')?;
        let port = match after.strip_prefix(':') {
            Some(p) => p.parse().ok()?,
            None if after.is_empty() => default,
            None => return None,
        };
        return Some((host.to_string(), port));
    }
    match s.rsplit_once(':') {
        Some((h, p)) => Some((h.to_string(), p.parse().ok()?)),
        None => Some((s.to_string(), default)),
    }
}

/// The proxy request line: `CONNECT host:port HTTP/1.1` or `GET http://host:port/path HTTP/1.1`.
pub fn parse_line(line: &str) -> Option<Target> {
    let mut parts = line.split_whitespace();
    let (method, target, version) = (parts.next()?, parts.next()?, parts.next()?);
    if !version.starts_with("HTTP/") {
        return None;
    }
    if method.eq_ignore_ascii_case("CONNECT") {
        let (host, port) = host_port(target, 443)?;
        return Some(Target { method: "CONNECT".into(), host, port, path: None });
    }
    let rest = target.strip_prefix("http://").or_else(|| target.strip_prefix("HTTP://"))?;
    let (authority, path) = match rest.find(['/', '?']) {
        Some(i) if rest.as_bytes()[i] == b'/' => (&rest[..i], rest[i..].to_string()),
        Some(i) => (&rest[..i], format!("/{}", &rest[i..])),
        None => (rest, "/".to_string()),
    };
    let authority = authority.rsplit('@').next().unwrap_or(authority);
    let (host, port) = host_port(authority, 80)?;
    Some(Target { method: method.to_string(), host, port, path: Some(path) })
}

enum Dest {
    Go(Vec<SocketAddr>),
    /// One of this machine's own addresses.
    Refused,
    /// The name doesn't resolve.
    Unknown,
}

/// Where to connect: the job server as asked, or the resolved addresses of anything else, unless any
/// of them is one of this machine's own (then the whole request is refused).
async fn destinations(host: &str, port: u16, jobs: SocketAddr) -> Dest {
    let host = host.trim_end_matches('.');
    if let Ok(ip) = host.parse::<IpAddr>() {
        let addr = SocketAddr::new(ip, port);
        if addr == jobs {
            return Dest::Go(vec![jobs]);
        }
        return if blocked(ip) { Dest::Refused } else { Dest::Go(vec![addr]) };
    }
    let addrs: Vec<SocketAddr> = match tokio::net::lookup_host((host, port)).await {
        Ok(a) => a.collect(),
        Err(_) => return Dest::Unknown,
    };
    if addrs.is_empty() {
        Dest::Unknown
    } else if addrs.iter().any(|a| blocked(a.ip())) {
        Dest::Refused
    } else {
        Dest::Go(addrs)
    }
}

/// Whether a web address's host name resolves (IP addresses always do).
pub async fn resolves(url: &str) -> bool {
    let rest = url.split_once("://").map(|(_, r)| r).unwrap_or(url);
    let Some(t) = parse_line(&format!("GET http://{rest} HTTP/1.1")) else { return false };
    let host = t.host.trim_end_matches('.');
    if host.parse::<IpAddr>().is_ok() {
        return true;
    }
    let found = tokio::net::lookup_host((host, t.port)).await;
    found.is_ok_and(|mut a| a.next().is_some())
}

/// Whether a web address points at this machine itself (for a clear message before printing).
pub async fn points_at_self(url: &str) -> bool {
    let rest = url.split_once("://").map(|(_, r)| r).unwrap_or(url);
    let Some(t) = parse_line(&format!("GET http://{rest} HTTP/1.1")) else { return false };
    let host = t.host.trim_end_matches('.');
    if let Ok(ip) = host.parse::<IpAddr>() {
        return blocked(ip);
    }
    let found = tokio::net::lookup_host((host, t.port)).await;
    found.is_ok_and(|mut addrs| addrs.any(|a| blocked(a.ip())))
}

/// Read from `c` into `buf` until it holds a complete head; returns the head's length.
async fn read_head(c: &mut TcpStream, buf: &mut Vec<u8>) -> io::Result<Option<usize>> {
    let mut chunk = [0u8; 4096];
    loop {
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            return Ok(Some(i + 4));
        }
        if buf.len() > MAX_HEAD {
            return Ok(None);
        }
        let n = c.read(&mut chunk).await?;
        if n == 0 {
            return Ok(None);
        }
        buf.extend_from_slice(&chunk[..n]);
    }
}

async fn connect(addrs: &[SocketAddr]) -> Option<TcpStream> {
    for a in addrs {
        if let Ok(Ok(s)) = tokio::time::timeout(Duration::from_secs(10), TcpStream::connect(a)).await {
            return Some(s);
        }
    }
    None
}

/// How a message body ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Framing {
    None,
    Length(u64),
    Chunked,
    /// Until the sender closes (responses only).
    Close,
}

fn header<'a>(head: &'a str, name: &str) -> Option<&'a str> {
    head.split("\r\n").skip(1).find_map(|l| {
        let (k, v) = l.split_once(':')?;
        k.trim().eq_ignore_ascii_case(name).then(|| v.trim())
    })
}

fn framing(head: &str, otherwise: Framing) -> Framing {
    if header(head, "transfer-encoding").is_some_and(|v| v.to_ascii_lowercase().contains("chunked")) {
        Framing::Chunked
    } else if let Some(n) = header(head, "content-length").and_then(|v| v.parse().ok()) {
        Framing::Length(n)
    } else {
        otherwise
    }
}

/// Make sure `buf` holds at least `n` bytes, reading more from `src`.
async fn fill(src: &mut TcpStream, buf: &mut Vec<u8>, n: usize) -> io::Result<()> {
    let mut chunk = [0u8; 8192];
    while buf.len() < n {
        let got = src.read(&mut chunk).await?;
        if got == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        buf.extend_from_slice(&chunk[..got]);
    }
    Ok(())
}

/// One line (with its CRLF) from the front of `buf`, reading more as needed.
async fn line(src: &mut TcpStream, buf: &mut Vec<u8>) -> io::Result<Vec<u8>> {
    loop {
        if let Some(i) = buf.windows(2).position(|w| w == b"\r\n") {
            return Ok(buf.drain(..i + 2).collect());
        }
        if buf.len() > MAX_HEAD {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let have = buf.len();
        fill(src, buf, have + 1).await?;
    }
}

/// Copy exactly `n` bytes (what's already in `buf` first, then straight from `src`), streaming.
async fn copy_exact(src: &mut TcpStream, buf: &mut Vec<u8>, dst: &mut TcpStream, n: u64) -> io::Result<()> {
    let first = n.min(buf.len() as u64) as usize;
    dst.write_all(&buf[..first]).await?;
    buf.drain(..first);
    let left = n - first as u64;
    if left > 0 {
        let copied = tokio::io::copy(&mut (&mut *src).take(left), dst).await?;
        if copied < left {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
    }
    Ok(())
}

/// Copy exactly one body, framed as `f`, from `src` (with `buf` already read) to `dst`. Anything after
/// it (such as a second, pipelined request) is not forwarded.
async fn copy_body(src: &mut TcpStream, buf: &mut Vec<u8>, dst: &mut TcpStream, f: Framing) -> io::Result<()> {
    match f {
        Framing::None => Ok(()),
        Framing::Length(n) => copy_exact(src, buf, dst, n).await,
        Framing::Chunked => loop {
            let size_line = line(src, buf).await?;
            let text = String::from_utf8_lossy(&size_line);
            let size = u64::from_str_radix(text.trim().split(';').next().unwrap_or("").trim(), 16)
                .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;
            dst.write_all(&size_line).await?;
            if size == 0 {
                // Trailers, up to the empty line.
                loop {
                    let t = line(src, buf).await?;
                    dst.write_all(&t).await?;
                    if t == b"\r\n" {
                        return Ok(());
                    }
                }
            }
            // Stream the chunk through as it arrives (never hold it all), then its closing CRLF.
            copy_exact(src, buf, dst, size).await?;
            let end = line(src, buf).await?;
            if end != b"\r\n" {
                return Err(io::ErrorKind::InvalidData.into());
            }
            dst.write_all(&end).await?;
        },
        Framing::Close => {
            dst.write_all(buf).await?;
            buf.clear();
            tokio::io::copy(src, dst).await?;
            Ok(())
        }
    }
}

/// Without these, a client or server could keep the connection for more requests.
fn hop_by_hop(name: &str) -> bool {
    matches!(name, "proxy-connection" | "connection" | "keep-alive" | "proxy-authorization" | "te" | "trailer" | "upgrade")
}

fn rewrite_headers(head: &str, first_line: &str) -> String {
    let mut out = format!("{first_line}\r\n");
    for l in head.split("\r\n").skip(1).filter(|l| !l.is_empty()) {
        let name = l.split(':').next().unwrap_or("").trim().to_ascii_lowercase();
        if !hop_by_hop(&name) {
            out.push_str(l);
            out.push_str("\r\n");
        }
    }
    out.push_str("Connection: close\r\nProxy-Connection: close\r\n\r\n");
    out
}

async fn handle(mut client: TcpStream, jobs: SocketAddr) -> io::Result<()> {
    let mut buf = Vec::with_capacity(4096);
    let Some(head_len) = read_head(&mut client, &mut buf).await? else { return Ok(()) };
    let head = String::from_utf8_lossy(&buf[..head_len]).into_owned();
    buf.drain(..head_len);
    let Some(target) = head.split("\r\n").next().and_then(parse_line) else {
        return client.write_all(FORBIDDEN).await;
    };
    let addrs = match destinations(&target.host, target.port, jobs).await {
        Dest::Go(a) => a,
        Dest::Refused => {
            tracing::info!("egress: refused {} {}:{}", target.method, target.host, target.port);
            return client.write_all(FORBIDDEN).await;
        }
        // Closing without a reply makes the browser treat it as a network error, not as a page.
        Dest::Unknown => return Ok(()),
    };
    let Some(mut upstream) = connect(&addrs).await else {
        return Ok(());
    };

    let Some(path) = target.path else {
        // CONNECT (HTTPS, wss): a tunnel to the one checked destination.
        client.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n").await?;
        upstream.write_all(&buf).await?;
        let _ = tokio::io::copy_bidirectional(&mut client, &mut upstream).await;
        return Ok(());
    };

    // Plain HTTP: exactly one request and one response, then the connection closes, so a second
    // request can never ride along to this destination; the client must come back and be checked.
    let version = head.split("\r\n").next().and_then(|l| l.split_whitespace().nth(2)).unwrap_or("HTTP/1.1");
    let request_head = rewrite_headers(&head, &format!("{} {} {}", target.method, path, version));
    upstream.write_all(request_head.as_bytes()).await?;
    copy_body(&mut client, &mut buf, &mut upstream, framing(&head, Framing::None)).await?;

    let mut rbuf = Vec::with_capacity(4096);
    loop {
        let Some(len) = read_head(&mut upstream, &mut rbuf).await? else { return Ok(()) };
        let rhead = String::from_utf8_lossy(&rbuf[..len]).into_owned();
        rbuf.drain(..len);
        let status_line = rhead.split("\r\n").next().unwrap_or("HTTP/1.1 502 Bad Gateway").to_string();
        let status: u16 = status_line.split_whitespace().nth(1).and_then(|c| c.parse().ok()).unwrap_or(502);
        client.write_all(rewrite_headers(&rhead, &status_line).as_bytes()).await?;
        if (100..200).contains(&status) {
            // An interim response (e.g. 100 Continue); the real one follows.
            continue;
        }
        let body = if target.method.eq_ignore_ascii_case("HEAD") || status == 204 || status == 304 {
            Framing::None
        } else {
            framing(&rhead, Framing::Close)
        };
        copy_body(&mut upstream, &mut rbuf, &mut client, body).await?;
        let _ = client.shutdown().await;
        return Ok(());
    }
}

/// Start the proxy on `bind` (e.g. "127.0.0.1:8092"); `jobs` is the job-file server Chromium may use.
pub async fn start_at(bind: &str, jobs: SocketAddr) -> io::Result<SocketAddr> {
    let listener = TcpListener::bind(bind).await?;
    let addr = listener.local_addr()?;
    tokio::spawn(async move {
        while let Ok((c, _)) = listener.accept().await {
            tokio::spawn(async move {
                let _ = handle(c, jobs).await;
            });
        }
    });
    Ok(addr)
}

/// Start on 127.0.0.1:8092, or any free port.
pub async fn start(jobs: SocketAddr) -> io::Result<SocketAddr> {
    match start_at("127.0.0.1:8092", jobs).await {
        Ok(a) => Ok(a),
        Err(_) => start_at("127.0.0.1:0", jobs).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn blocked_addresses() {
        for s in ["127.0.0.1", "127.9.9.9", "0.0.0.0", "0.1.2.3", "169.254.169.254", "::1", "::", "::ffff:127.0.0.1", "::127.0.0.1", "fe80::1", "::ffff:169.254.169.254"] {
            assert!(blocked(ip(s)), "{s} should be blocked");
        }
        for s in ["192.168.1.10", "10.0.0.5", "172.18.0.3", "8.8.8.8", "2606:4700::1111", "fd00::5"] {
            assert!(!blocked(ip(s)), "{s} should be allowed");
        }
    }

    #[tokio::test]
    async fn addresses_that_point_at_this_machine() {
        assert!(points_at_self("http://localhost:8080/").await);
        assert!(points_at_self("HTTPS://127.0.0.1/").await);
        assert!(points_at_self("http://[::ffff:127.0.0.1]/").await);
        assert!(!points_at_self("http://10.0.0.5:3000").await);
        assert!(!points_at_self("http://no-such-host.invalid/").await);
    }

    #[tokio::test]
    async fn names_that_resolve() {
        assert!(resolves("http://localhost/").await);
        assert!(resolves("http://10.0.0.5:3000/").await);
        assert!(!resolves("http://no-such-host.invalid/").await);
    }

    #[test]
    fn request_lines() {
        assert_eq!(parse_line("CONNECT example.com:443 HTTP/1.1"), Some(Target { method: "CONNECT".into(), host: "example.com".into(), port: 443, path: None }));
        assert_eq!(
            parse_line("GET http://wiki.lan:8080/a?b=1 HTTP/1.1"),
            Some(Target { method: "GET".into(), host: "wiki.lan".into(), port: 8080, path: Some("/a?b=1".into()) })
        );
        assert_eq!(parse_line("POST http://[::1]/x HTTP/1.1").map(|t| (t.host, t.port)), Some(("::1".into(), 80)));
        assert_eq!(parse_line("GET http://h HTTP/1.1").and_then(|t| t.path), Some("/".into()));
        assert_eq!(parse_line("GET /relative HTTP/1.1"), None);
        assert_eq!(parse_line("GET ftp://h/x HTTP/1.1"), None);
    }
}
