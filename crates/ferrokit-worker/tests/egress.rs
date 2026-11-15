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
