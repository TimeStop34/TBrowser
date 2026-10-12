use std::collections::HashMap;

use anyhow::{bail, Context};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use url::Url;

use crate::network::base::Response;

pub struct UtnpClient;

impl UtnpClient {
    pub async fn fetch(url: &str) -> anyhow::Result<Response> {
        let parsed = Url::parse(url).context("invalid URL")?;
        let host = parsed.host_str().unwrap_or("localhost");
        let port = parsed.port().unwrap_or(34);
        let path = parsed.path().trim_start_matches('/');

        if path.is_empty() {
            bail!("Missing command in UTNP URL");
        }
        let (command, arg) = match path.split_once('/') {
            Some((c, a)) => (c.to_ascii_lowercase(), Some(a.to_string())),
            None => (path.to_ascii_lowercase(), None),
        };

        if command != "list" && command != "negotiate" {
            bail!("Unknown UTNP command: {}", command);
        }
        if command == "negotiate" && arg.is_none() {
            bail!("negotiate requires protocol list");
        }

        let mut req = format!("UTNP/1.0 {}", command.to_ascii_uppercase());
        if let Some(a) = &arg {
            req.push(' ');
            req.push_str(a);
        }
        req.push('\n');

        // Подключаемся
        let mut sock = TcpStream::connect((host, port)).await
            .with_context(|| format!("connect {}:{}", host, port))?;
        sock.write_all(req.as_bytes()).await?;

        // Читаем до EOF (сервер закрывает соединение — как в Python-цикле recv)
        let mut buf = Vec::new();
        sock.read_to_end(&mut buf).await?;

        let text = String::from_utf8_lossy(&buf).into_owned();
        let mut lines = text.lines();
        let status_line = lines
            .next()
            .map(str::to_string)
            .ok_or_else(|| anyhow::anyhow!("Empty UTNP response"))?;
        let body = lines.collect::<Vec<_>>().join("\n");

        let code: u16 = if status_line.starts_with("UTNP/1.0 START PROTOCOL")
            || status_line.starts_with("UTNP/1.0 AVAILABLE")
        {
            20
        } else {
            30
        };
        let name = if code == 20 { "OK" } else { "ERROR" }.to_string();

        Ok(Response {
            status_line,
            status_code: code,
            status_name: name,
            headers: HashMap::new(),
            body: body.clone().into_bytes(),
            text: body,
        })
    }
}

pub async fn negotiate_protocol(
    host: &str,
    protocol: &str,
    utnp_port: u16,
) -> anyhow::Result<(String, u16)> {
    let url = format!("utnp://{}:{}/negotiate/{}", host, utnp_port, protocol);
    let resp = UtnpClient::fetch(&url).await?;

    if !resp.ok() {
        bail!(
            "UTNP negotiation failed: {} {}",
            resp.status_code,
            resp.status_name
        );
    }

    // START PROTOCOL в статусной строке
    if resp.status_line.starts_with("UTNP/1.0 START PROTOCOL") {
        let parts: Vec<&str> = resp.status_line.split_whitespace().collect();
        if parts.len() >= 7 && parts[3] == protocol {
            let proto_host = parts[5].to_string();
            let proto_port: u16 = parts[6].parse().context("bad port in status line")?;
            return Ok((proto_host, proto_port));
        }
    }

    // Иначе ищем в теле (AVAILABLE)
    for line in resp.text.lines() {
        let line = line.trim();
        if line.starts_with(protocol) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 && parts[0] == protocol {
                let proto_host = parts[2].to_string();
                let proto_port: u16 = parts[3].parse().context("bad port in body")?;
                return Ok((proto_host, proto_port));
            }
        }
    }

    bail!("Protocol {} not found in UTNP response", protocol);
}