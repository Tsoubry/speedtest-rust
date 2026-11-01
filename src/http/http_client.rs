use std::{
    io::{Error, Result as IoResult},
    pin::Pin,
    task::{Context, Poll},
};

use serde_json::Value;
use tokio::{
    io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _, ReadBuf},
    net::TcpStream,
};
use tokio_rustls::client::TlsStream;

use crate::http::tls::setup_tls_connector;

#[derive(Debug)]
pub enum ClientStream {
    Tcp(TcpStream),
    Tls(Box<TlsStream<TcpStream>>),
}

pub struct HttpClient {
    pub stream: ClientStream,
}

impl AsyncRead for ClientStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<IoResult<()>> {
        match self.get_mut() {
            Self::Tcp(tcp) => Pin::new(tcp).poll_read(cx, buf),
            Self::Tls(tls) => Pin::new(tls).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for ClientStream {
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        match self.get_mut() {
            Self::Tcp(tcp) => Pin::new(tcp).poll_flush(cx),
            Self::Tls(tls) => Pin::new(tls).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        match self.get_mut() {
            Self::Tcp(tcp) => Pin::new(tcp).poll_flush(cx),
            Self::Tls(tls) => Pin::new(tls).poll_flush(cx),
        }
    }

    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, Error>> {
        match self.get_mut() {
            Self::Tcp(tcp) => Pin::new(tcp).poll_write(cx, buf),
            Self::Tls(tls) => Pin::new(tls).poll_write(cx, buf),
        }
    }
}

impl HttpClient {
    pub async fn open(url: &str) -> IoResult<Self> {
        let pared_url = Self::parse_url(url)?;
        let tcp_stream = TcpStream::connect(format!("{}:{}", pared_url.1, pared_url.2)).await?;
        let stream = if pared_url.2 == 443 {
            let tls_stream = setup_tls_connector(pared_url.1.clone(), tcp_stream).await?;
            ClientStream::Tls(Box::from(tls_stream))
        } else {
            ClientStream::Tcp(tcp_stream)
        };
        Ok(Self { stream })
    }

    fn parse_url(url: &str) -> IoResult<(String, String, i32)> {
        if let Some((scheme, rest)) = url.split_once("://") {
            let host = rest.split_once('/').map_or(rest, |(host, _)| host);
            let port = if scheme == "https" { 443 } else { 80 };
            Ok((scheme.to_owned(), host.to_owned(), port))
        } else {
            Err(Error::other("Error parsing input url"))
        }
    }

    pub async fn send_request_json(&mut self, packet: &[u8]) -> IoResult<Option<Value>> {
        self.stream.write_all(packet).await?;
        let mut read_data = Vec::new();
        loop {
            let mut response = vec![0; 128];
            let read = self.stream.read(&mut response).await?;
            read_data.extend(response);
            if read < 128 {
                //EOF
                break;
            }
        }
        let parser = String::from_utf8_lossy(&read_data);
        let response = parser.trim_matches(char::from(0));
        if response.starts_with("HTTP/1.1 200") {
            let mut split_body = response.splitn(2, "\r\n\r\n");
            let resp_body = split_body.nth(1).unwrap_or("");
            Ok(serde_json::from_str::<Value>(resp_body).ok())
        } else {
            Ok(None)
        }
    }
}
