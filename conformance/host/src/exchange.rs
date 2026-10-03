//! One HTTP/1.1 exchange over a raw `TcpStream` (ADR 30.9.26al D6).
//!
//! Written by hand rather than through an HTTP client for two reasons the
//! contract cares about. A kit may answer `413` from `Content-Length` and
//! close before the body is written, so the response is read **while** the
//! body is written, and a write error is ignored once a response has
//! arrived. And `header_case: title` must reach the app as typed, which a
//! client that normalises header names cannot promise. The reply is timed
//! from the first byte written to the last body byte read, as the relay's
//! budget is.

use std::time::{Duration, Instant};

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

use crate::case::{HeaderCase, Method};
use crate::sign::Built;

/// What came back.
#[derive(Debug)]
pub struct Reply {
    pub status: u16,
    /// Lowercase names.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub elapsed: Duration,
}

impl Reply {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str())
    }
}

/// `webhook-id` → `Webhook-Id`.
pub fn title_case(name: &str) -> String {
    name.split('-')
        .map(|part| {
            let mut c = part.chars();
            c.next().map(|f| f.to_ascii_uppercase().to_string() + c.as_str()).unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join("-")
}

/// The request's bytes up to and including the blank line.
pub fn head(built: &Built, port: u16, case: HeaderCase) -> Vec<u8> {
    let method = match built.method {
        Method::Post => "POST",
        Method::Get => "GET",
        Method::Put => "PUT",
    };
    let mut lines = vec![format!("{method} / HTTP/1.1")];
    let host = format!("127.0.0.1:{port}");
    let length = built.body.len().to_string();
    let framing = [("host", host.as_str()), ("content-length", length.as_str()), ("connection", "close")];
    let named = built.headers.iter().map(|(n, v)| (*n, v.as_str())).chain(framing);
    for (name, value) in named {
        let name = if case == HeaderCase::Title { title_case(name) } else { name.to_string() };
        lines.push(format!("{name}: {value}"));
    }
    (lines.join("\r\n") + "\r\n\r\n").into_bytes()
}

/// Sends `built` to `127.0.0.1:port` and reads the reply, giving up at
/// `limit`.
pub async fn exchange(built: &Built, port: u16, case: HeaderCase, limit: Duration) -> Result<Reply, String> {
    let stream = TcpStream::connect(("127.0.0.1", port)).await.map_err(|e| format!("connect: {e}"))?;
    let (read, mut write) = stream.into_split();
    let mut request = head(built, port, case);
    request.extend_from_slice(&built.body);
    let start = Instant::now();
    let send = async move {
        // A kit that has already answered may close before reading the
        // body; the reply decides, so the write's own result is dropped.
        let _ = write.write_all(&request).await;
        let _ = write.flush().await;
        write
    };
    let receive = read_reply(BufReader::new(read));
    let (_, reply) = tokio::time::timeout(limit, async { tokio::join!(send, receive) })
        .await
        .map_err(|_| format!("no complete reply within {} ms", limit.as_millis()))?;
    let (status, headers, body) = reply?;
    Ok(Reply { status, headers, body, elapsed: start.elapsed() })
}

type Parts = (u16, Vec<(String, String)>, Vec<u8>);

async fn read_reply<R: tokio::io::AsyncRead + Unpin>(mut r: BufReader<R>) -> Result<Parts, String> {
    let mut line = String::new();
    r.read_line(&mut line).await.map_err(|e| format!("read status: {e}"))?;
    let status = line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| format!("not an HTTP status line: {line:?}"))?;
    let mut headers = Vec::new();
    loop {
        line.clear();
        r.read_line(&mut line).await.map_err(|e| format!("read header: {e}"))?;
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            break;
        }
        let (name, value) = trimmed.split_once(':').ok_or_else(|| format!("not a header: {trimmed:?}"))?;
        headers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
    }
    let find = |n: &str| headers.iter().find(|(k, _)| k == n).map(|(_, v)| v.clone());
    let body = if find("transfer-encoding").is_some_and(|v| v.eq_ignore_ascii_case("chunked")) {
        read_chunked(&mut r).await?
    } else if let Some(length) = find("content-length") {
        let length: usize = length.parse().map_err(|_| format!("bad content-length {length:?}"))?;
        let mut body = vec![0; length];
        r.read_exact(&mut body).await.map_err(|e| format!("read body: {e}"))?;
        body
    } else {
        let mut body = Vec::new();
        r.read_to_end(&mut body).await.map_err(|e| format!("read body: {e}"))?;
        body
    };
    Ok((status, headers, body))
}

async fn read_chunked<R: tokio::io::AsyncRead + Unpin>(r: &mut BufReader<R>) -> Result<Vec<u8>, String> {
    let mut body = Vec::new();
    let mut line = String::new();
    loop {
        line.clear();
        r.read_line(&mut line).await.map_err(|e| format!("read chunk size: {e}"))?;
        let size_hex = line.trim().split(';').next().unwrap_or_default();
        let size = usize::from_str_radix(size_hex, 16).map_err(|_| format!("bad chunk size {line:?}"))?;
        if size == 0 {
            return Ok(body);
        }
        let mut chunk = vec![0; size + 2];
        r.read_exact(&mut chunk).await.map_err(|e| format!("read chunk: {e}"))?;
        body.extend_from_slice(&chunk[..size]);
    }
}
