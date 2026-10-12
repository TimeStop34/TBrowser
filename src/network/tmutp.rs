use std::collections::HashMap;

use anyhow::{bail, Context};
use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use url::Url;

use crate::network::base::{
    FetchOptions, PostCompatibleProtocol, ProgressCallback, Response, TextProtocol,
};
use crate::network::utnp::negotiate_protocol;

pub struct TmutpClient;

#[async_trait]
impl TextProtocol for TmutpClient {
    async fn fetch(
        &self,
        url: &str,
        progress: Option<&ProgressCallback>,
        options: &FetchOptions,
    ) -> anyhow::Result<Response> {
        let (host, port, path) = resolve(url).await?;
        let req = format!("TMUTP/1.0 GET {}\r\n\r\n", path);
        send_request(
            &host,
            port,
            &req,
            progress,
            options.follow_redirects,
            options.max_redirects,
            url,
        )
        .await
    }
}

#[async_trait]
impl PostCompatibleProtocol for TmutpClient {
    async fn post(
        &self,
        url: &str,
        data: &[(&str, &str)],
        progress: Option<&ProgressCallback>,
    ) -> anyhow::Result<Response> {
        let (host, port, path) = resolve(url).await?;

        let body: String = form_urlencoded::Serializer::new(String::new())
            .extend_pairs(data.iter().copied())
            .finish();

        let req = format!(
            "TMUTP/1.0 POST {}\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{}",
            path,
            body.len(),
            body
        );

        send_request(&host, port, &req, progress, true, 10, url).await
    }
}

async fn resolve(url: &str) -> anyhow::Result<(String, u16, String)> {
    let parsed = Url::parse(url).context("invalid URL")?;
    let mut host = parsed.host_str().unwrap_or("localhost").to_string();
    let mut port = parsed.port();
    if port.is_none() {
        let (h, p) = negotiate_protocol(&host, "tmutp", 34).await?;
        host = h;
        port = Some(p);
    }
    let mut path = if parsed.path().is_empty() {
        "/".to_string()
    } else {
        parsed.path().to_string()
    };
    if let Some(q) = parsed.query() {
        path.push('?');
        path.push_str(q);
    }
    Ok((host, port.unwrap(), path))
}

async fn send_request(
    host: &str,
    port: u16,
    req: &str,
    progress: Option<&ProgressCallback>,
    follow_redirects: bool,
    max_redirects: u32,
    original_url: &str,
) -> anyhow::Result<Response> {
    let mut sock = TcpStream::connect((host, port)).await?;
    sock.write_all(req.as_bytes()).await?;

    let mut reader = BufReader::new(sock);

    let mut line = String::new();
    reader.read_line(&mut line).await?;
    let status_line = line.trim_end_matches(['\r', '\n']).to_string();
    if status_line.is_empty() {
        bail!("Empty response");
    }
    let parts: Vec<&str> = status_line.split_whitespace().collect();
    if parts.len() < 3 {
        bail!("Malformed status line");
    }
    let code: u16 = parts[1].parse().context("bad status code")?;
    let name = parts[2..].join(" ");

    let mut headers: HashMap<String, String> = HashMap::new();
    let mut content_length: u64 = 0;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        let l = line.trim_end_matches(['\r', '\n']);
        if l.is_empty() {
            break;
        }
        if let Some((k, v)) = l.split_once(':') {
            let k = k.trim().to_ascii_lowercase();
            let v = v.trim().to_string();
            if k == "content-length" {
                content_length = v.parse().context("bad content-length")?;
            }
            headers.insert(k, v);
        }
    }

    let mut body = Vec::new();
    if content_length > 0 {
        let mut total_read: u64 = 0;
        let mut buf = vec![0u8; 8192];
        while total_read < content_length {
            let to_read = std::cmp::min(8192, (content_length - total_read) as usize);
            let n = reader.read(&mut buf[..to_read]).await?;
            if n == 0 {
                break;
            }
            body.extend_from_slice(&buf[..n]);
            total_read += n as u64;
            if let Some(cb) = progress {
                cb(total_read, content_length);
            }
        }
    } else {
        reader.read_to_end(&mut body).await?;
    }

    let text = String::from_utf8_lossy(&body).into_owned();

    if follow_redirects && code == 21 && max_redirects > 0 {
        if let Some(location) = headers.get("location").cloned() {
            let base = Url::parse(original_url)?;
            let new_url = base.join(&location)?.to_string();
            let (h, p, path) = resolve(&new_url).await?;
            let req = format!("TMUTP/1.0 GET {}\r\n\r\n", path);
            return Box::pin(send_request(
                &h,
                p,
                &req,
                progress,
                follow_redirects,
                max_redirects - 1,
                &new_url,
            ))
            .await;
        }
    }

    headers
        .entry("content-type".to_string())
        .or_insert_with(|| "text/tmkup".to_string());

    Ok(Response {
        status_line,
        status_code: code,
        status_name: name,
        headers,
        body,
        text,
    })
}
