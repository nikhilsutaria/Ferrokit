//! The egress proxy every Chromium request goes through: this machine's own services stay out of reach,
//! whatever a page writes; the job-file server and ordinary network addresses are let through.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use ferrokit_worker::egress;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// A server that answers every request with "ok" and counts its connections.
async fn server(bind: &str) -> (SocketAddr, Arc<AtomicUsize>) {
    let listener = TcpListener::bind(bind).await.unwrap();
    let addr = listener.local_addr().unwrap();
    let hits = Arc::new(AtomicUsize::new(0));
    let h = hits.clone();
    tokio::spawn(async move {
        while let Ok((mut c, _)) = listener.accept().await {
            h.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                let _ = c.read(&mut buf).await;
                let _ = c.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok").await;
            });
        }
    });
    (addr, hits)
}

/// Send a raw request to the proxy and return the whole reply.
async fn ask(proxy: SocketAddr, head: &str) -> String {
    let mut c = TcpStream::connect(proxy).await.unwrap();
    c.write_all(head.as_bytes()).await.unwrap();
    let mut out = Vec::new();
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), c.read_to_end(&mut out)).await;
    String::from_utf8_lossy(&out).into_owned()
}

/// This machine's address on its network (not loopback), found without sending anything.
fn lan_ip() -> std::net::IpAddr {
    let s = std::net::UdpSocket::bind("0.0.0.0:0").unwrap();
    s.connect("10.255.255.1:9").unwrap();
    s.local_addr().unwrap().ip()
}

#[tokio::test]
async fn loopback_is_refused_in_every_form() {
    let (internal, hits) = server("127.0.0.1:0").await;
    let (jobs, _) = server("127.0.0.1:0").await;
    let proxy = egress::start_at("127.0.0.1:0", jobs).await.unwrap();
    let p = internal.port();

    for head in [
        format!("GET http://127.0.0.1:{p}/ HTTP/1.1\r\nHost: 127.0.0.1:{p}\r\n\r\n"),
        format!("GET http://localhost:{p}/ HTTP/1.1\r\nHost: localhost:{p}\r\n\r\n"),
        format!("GET http://[::ffff:127.0.0.1]:{p}/ HTTP/1.1\r\nHost: x\r\n\r\n"),
        format!("GET http://[::ffff:7f00:1]:{p}/ HTTP/1.1\r\nHost: x\r\n\r\n"),
        format!("GET http://0.0.0.0:{p}/ HTTP/1.1\r\nHost: x\r\n\r\n"),
        format!("CONNECT 127.0.0.1:{p} HTTP/1.1\r\nHost: 127.0.0.1:{p}\r\n\r\n"),
        format!("CONNECT localhost:{p} HTTP/1.1\r\n\r\n"),
    ] {
        let reply = ask(proxy, &head).await;
        assert!(reply.starts_with("HTTP/1.1 403"), "{head:?} → {reply:?}");
    }
    assert_eq!(hits.load(Ordering::SeqCst), 0, "the internal service was reached");
}

#[tokio::test]
async fn job_server_and_network_addresses_pass() {
    let (jobs, _) = server("127.0.0.1:0").await;
    let (lan, lan_hits) = server(&format!("{}:0", lan_ip())).await;
    let proxy = egress::start_at("127.0.0.1:0", jobs).await.unwrap();

    let reply = ask(proxy, &format!("GET http://127.0.0.1:{}/job/x/index.html HTTP/1.1\r\nHost: x\r\n\r\n", jobs.port())).await;
    assert!(reply.starts_with("HTTP/1.1 200") && reply.ends_with("ok"), "{reply:?}");

    let reply = ask(proxy, &format!("GET http://{lan}/page HTTP/1.1\r\nHost: {lan}\r\nProxy-Connection: keep-alive\r\n\r\n")).await;
    assert!(reply.starts_with("HTTP/1.1 200") && reply.ends_with("ok"), "{reply:?}");

    let reply = ask(proxy, &format!("CONNECT {lan} HTTP/1.1\r\nHost: {lan}\r\n\r\n")).await;
    assert!(reply.starts_with("HTTP/1.1 200"), "{reply:?}");
    assert_eq!(lan_hits.load(Ordering::SeqCst), 2);
}

/// A keep-alive server that records every byte it receives and answers the first request.
async fn recording_server(bind: &str, reply: &'static [u8]) -> (SocketAddr, Arc<std::sync::Mutex<Vec<u8>>>) {
    let listener = TcpListener::bind(bind).await.unwrap();
    let addr = listener.local_addr().unwrap();
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let s = seen.clone();
    tokio::spawn(async move {
        while let Ok((mut c, _)) = listener.accept().await {
            let s = s.clone();
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                let mut answered = false;
                // Keep the connection open and keep listening, as a keep-alive server would.
                while let Ok(Ok(n)) = tokio::time::timeout(std::time::Duration::from_millis(800), c.read(&mut buf)).await {
                    if n == 0 {
                        break;
                    }
                    s.lock().unwrap().extend_from_slice(&buf[..n]);
                    if !answered {
                        answered = true;
                        let _ = c.write_all(reply).await;
                    }
                }
            });
        }
    });
    (addr, seen)
}

/// Two requests on one proxy connection: only the first is sent, to its own host; the second, aimed at
/// this machine's internal service, goes nowhere. The client is told the connection closes.
#[tokio::test]
async fn one_request_per_connection() {
    let (internal, hits) = server("127.0.0.1:0").await;
    let (jobs, _) = server("127.0.0.1:0").await;
    let (lan, seen) = recording_server(&format!("{}:0", lan_ip()), b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: keep-alive\r\n\r\nok").await;
    let proxy = egress::start_at("127.0.0.1:0", jobs).await.unwrap();

    let two = format!(
        "POST http://{lan}/first HTTP/1.1\r\nHost: {lan}\r\nContent-Length: 5\r\n\r\nhello\
         GET http://127.0.0.1:{}/second HTTP/1.1\r\nHost: x\r\nCookie: secret=1\r\n\r\n",
        internal.port()
    );
    let reply = ask(proxy, &two).await;
    assert!(reply.starts_with("HTTP/1.1 200"), "{reply:?}");
    assert!(reply.to_ascii_lowercase().contains("connection: close"), "{reply:?}");
    assert!(reply.ends_with("ok"), "{reply:?}");

    let got = String::from_utf8_lossy(&seen.lock().unwrap()).into_owned();
    assert!(got.contains("POST /first") && got.contains("hello"), "{got:?}");
    assert!(!got.contains("/second") && !got.contains("secret=1"), "the second request leaked upstream: {got:?}");
    assert_eq!(hits.load(Ordering::SeqCst), 0);
}

/// A chunked response is passed on whole, and the connection then ends.
#[tokio::test]
async fn chunked_responses_are_relayed_whole() {
    let (jobs, _) = server("127.0.0.1:0").await;
    let (lan, _) = recording_server(
        &format!("{}:0", lan_ip()),
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n3\r\nabc\r\n2\r\nde\r\n0\r\n\r\n",
    )
    .await;
    let proxy = egress::start_at("127.0.0.1:0", jobs).await.unwrap();
    let t = std::time::Instant::now();
    let reply = ask(proxy, &format!("GET http://{lan}/ HTTP/1.1\r\nHost: {lan}\r\n\r\n")).await;
    assert!(reply.ends_with("3\r\nabc\r\n2\r\nde\r\n0\r\n\r\n"), "{reply:?}");
    assert!(t.elapsed() < std::time::Duration::from_secs(2), "the connection should end with the response");
}

/// A huge chunk is passed on as it arrives, not held in memory until it is complete.
#[tokio::test]
async fn chunks_stream_through() {
    let (jobs, _) = server("127.0.0.1:0").await;
    let listener = TcpListener::bind(format!("{}:0", lan_ip())).await.unwrap();
    let lan = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut c, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 4096];
        let _ = c.read(&mut buf).await;
        // Announce 10 MB, send 1 MB, then pause.
        c.write_all(format!("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n{:x}\r\n", 10 * 1024 * 1024).as_bytes()).await.unwrap();
        c.write_all(&vec![b'x'; 1024 * 1024]).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    });
    let proxy = egress::start_at("127.0.0.1:0", jobs).await.unwrap();
    let mut c = TcpStream::connect(proxy).await.unwrap();
    c.write_all(format!("GET http://{lan}/ HTTP/1.1\r\nHost: {lan}\r\n\r\n").as_bytes()).await.unwrap();
    let mut got = 0usize;
    let mut buf = vec![0u8; 65536];
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(1500);
    while let Ok(Ok(n)) = tokio::time::timeout_at(deadline, c.read(&mut buf)).await {
        if n == 0 {
            break;
        }
        got += n;
    }
    assert!(got > 1024 * 1024, "only {got} bytes arrived while the server paused");
}

/// A nonsense chunk size is refused without trouble, and the proxy keeps working.
#[tokio::test]
async fn absurd_chunk_sizes_are_refused() {
    let (jobs, _) = server("127.0.0.1:0").await;
    let (lan, _) = recording_server(&format!("{}:0", lan_ip()), b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nFFFFFFFFFFFFFFFFFFFF\r\nab").await;
    let proxy = egress::start_at("127.0.0.1:0", jobs).await.unwrap();
    let reply = ask(proxy, &format!("GET http://{lan}/ HTTP/1.1\r\nHost: {lan}\r\n\r\n")).await;
    assert!(!reply.contains("ab\r\n") || reply.len() < 400, "{reply:?}");
    let reply = ask(proxy, &format!("GET http://127.0.0.1:{}/ HTTP/1.1\r\nHost: x\r\n\r\n", jobs.port())).await;
    assert!(reply.ends_with("ok"), "the proxy stopped working: {reply:?}");
}
