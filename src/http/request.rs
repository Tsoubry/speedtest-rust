use std::{collections::HashMap, future::Future, pin::Pin};

use case_insensitive_hashmap::CaseInsensitiveHashMap as CIHashMap;
use log::trace;
use tokio::io::{AsyncBufReadExt as _, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};

use crate::{
    config::GARBAGE_DATA,
    http::{Method, MethodStr as _, response::Response},
};

#[derive(Debug)]
enum BodyType {
    Chunked,
    Fixed(u64),
}

#[derive(Debug)]
pub struct Request {
    pub method: Method,
    pub path: String,
    pub query_params: HashMap<String, String>,
    pub remote_addr: String,
}

fn check_has_body(headers: &CIHashMap<String>) -> Option<BodyType> {
    if let Some(content_len) = headers.get("Content-Length") {
        let parsed_len = content_len.parse::<u64>().unwrap_or(0);
        if parsed_len > 0 {
            return Some(BodyType::Fixed(parsed_len));
        }
    }
    if headers.get("Transfer-Encoding").is_some_and(|te| te == "chunked") {
        return Some(BodyType::Chunked);
    }
    None
}

//allow http 1.* & POST, GET, OPTIONS methods
fn check_is_status_line(line: &str) -> bool {
    line.contains("http/1.") && (line.starts_with("get") || line.starts_with("options") || line.starts_with("post"))
}

fn clear_path_end_slash(input: &str) -> &str {
    input.strip_suffix('/').unwrap_or(input)
}

pub async fn handle_socket<R, W, F>(
    remote_addr: &str,
    buf_reader: &mut BufReader<R>,
    buf_writer: &mut BufWriter<W>,
    result: F,
) where
    R: AsyncReadExt + Unpin,
    W: AsyncWriteExt + Unpin,
    F: Send + Sync + Fn(Request) -> Pin<Box<dyn Future<Output = Response> + Send>>, {
    'root_loop: loop {
        //read status line
        let parsed_status = {
            if let Ok(Some(status_line)) = buf_reader.lines().next_line().await {
                if check_is_status_line(&status_line.to_lowercase()) {
                    let Some(parsed) = parse_request_status_line(&status_line) else {
                        break 'root_loop;
                    };
                    parsed
                } else {
                    break 'root_loop;
                }
            } else {
                break 'root_loop;
            }
        };
        //read headers
        let parsed_headers = header_parser(buf_reader).await;
        //honor `Expect: 100-continue` (RFC 9110 §10.1.1) by sending the interim 100 response
        //*before* reading the body. Without it, clients that use Expect (curl and many HTTP
        //libraries) wait out their continue-timeout (~1s for curl) on every upload request.
        if let Some(expect) = parsed_headers.get("Expect")
            && expect.eq_ignore_ascii_case("100-continue")
            && buf_writer.write_all(b"HTTP/1.1 100 Continue\r\n\r\n").await.is_ok()
            && let Err(err) = buf_writer.flush().await
        {
            trace!("Error socket flush : {err}");
        }
        //read and discard body content
        match check_has_body(&parsed_headers) {
            Some(BodyType::Fixed(body_size)) => {
                // Read exactly `body_size` bytes, counting the bytes actually returned, so any
                // Content-Length is safe (no blocking on a partial trailing chunk).
                let mut buffer = vec![0_u8; 65536];
                let mut remaining = body_size;
                while remaining > 0 {
                    let want = usize::try_from(remaining).map_or(buffer.len(), |rem| rem.min(buffer.len()));
                    let Some(slice) = buffer.get_mut(..want) else {
                        break;
                    };
                    match buf_reader.read(slice).await {
                        Ok(0) | Err(_) => break, // peer closed before sending it all
                        Ok(read) => remaining -= u64::try_from(read).unwrap_or(remaining),
                    }
                }
            }
            Some(BodyType::Chunked) => {
                let mut buffer = [0; 1024];
                while let Ok(read) = buf_reader.read_exact(&mut buffer).await {
                    if read == 0 {
                        break;
                    }
                }
            }
            None => {}
        }
        //trust proxy
        let client_addr = trust_addr_proxy(&parsed_headers, remote_addr);
        //gen request
        let response = result(Request {
            method: parsed_status.0,
            path: parsed_status.1,
            query_params: parsed_status.2,
            remote_addr: client_addr,
        })
        .await;
        if let Err(err) = buf_writer.write_all(&response.data).await {
            trace!("Error socket write : {err}");
        }
        if response.chunk_count > 0 {
            if let Some(garbage) = GARBAGE_DATA.get() {
                for _ in 0..response.chunk_count {
                    if let Err(err) = buf_writer.write_all(garbage).await {
                        trace!("Error socket write chunk : {err}");
                    }
                }
            }
            if let Err(err) = buf_writer.write_all(b"0\r\n\r\n").await {
                trace!("Error socket write eof : {err}");
            }
        }
        if let Err(err) = buf_writer.flush().await {
            trace!("Error socket flush : {err}");
        }
    }
}

pub async fn header_parser<R>(buf_reader: &mut BufReader<R>) -> CIHashMap<String>
where
    R: AsyncReadExt + Unpin, {
    let mut headers_out = CIHashMap::new();
    'header_loop: loop {
        if let Ok(Some(header_line)) = buf_reader.lines().next_line().await {
            if header_line.is_empty() {
                break 'header_loop;
            }
            let mut header_parts = header_line.splitn(2, ':');
            if let (Some(header_key), Some(header_val)) = (header_parts.next(), header_parts.next()) {
                let _ = headers_out.insert(header_key.trim().to_owned(), header_val.trim().to_owned());
            }
        } else {
            break 'header_loop;
        }
    }
    headers_out
}

fn parse_raw_path(raw_path: &str) -> (&str, HashMap<String, String>) {
    let mut real_path = raw_path;
    let mut query_params = HashMap::new();
    if raw_path.contains('?') {
        let mut split_raw_path = raw_path.split('?');
        real_path = clear_path_end_slash(split_raw_path.next().unwrap_or_default());
        let raw_query_params = split_raw_path.next().unwrap_or_default();
        for part in raw_query_params.split('&') {
            let mut split_part = part.split('=');
            if let (Some(query_key), Some(query_val)) = (split_part.next(), split_part.next()) {
                let _ = query_params.insert(query_key.to_owned(), query_val.to_owned());
            }
        }
    }
    (real_path, query_params)
}

fn parse_request_status_line(line: &str) -> Option<(Method, String, HashMap<String, String>)> {
    let mut split_status = line.split(' ');
    let method_str = split_status.next()?;
    let raw_path = split_status.next()?;
    let (path, query_params) = parse_raw_path(raw_path);
    Some((method_str.to_method(), path.to_owned(), query_params))
}

fn trust_addr_proxy(headers: &CIHashMap<String>, remote_addr: &str) -> String {
    headers
        .get("X-Real-IP")
        .map(String::as_str)
        .or_else(|| {
            headers
                .get("X-Forwarded-For")
                .and_then(|forwarded| forwarded.split(',').next().map(str::trim))
        })
        .unwrap_or(remote_addr)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Result as IoResult};

    use super::*;

    fn count_occurrences(haystack: &[u8], needle: &[u8]) -> usize {
        if needle.is_empty() {
            return 0;
        }
        haystack
            .windows(needle.len())
            .filter(|window| *window == needle)
            .count()
    }

    async fn drive(raw: Vec<u8>) -> IoResult<Vec<u8>> {
        let mut reader = BufReader::new(Cursor::new(raw));
        let mut writer = BufWriter::new(Vec::new());
        handle_socket("127.0.0.1", &mut reader, &mut writer, ok_router).await;
        writer.flush().await?;
        Ok(writer.into_inner())
    }

    /// `Expect: 100-continue` must get an interim `100 Continue` so the client sends its body
    /// immediately instead of waiting out its continue-timeout (~1s for curl) on every upload.
    #[tokio::test]
    async fn expect_100_continue_receives_interim_response() -> IoResult<()> {
        let mut raw = Vec::new();
        raw.extend_from_slice(b"POST /empty HTTP/1.1\r\nExpect: 100-continue\r\nContent-Length: 2048\r\n\r\n");
        raw.extend_from_slice(&vec![b'x'; 2048]);
        let out = drive(raw).await?;
        assert!(
            count_occurrences(&out, b"100 Continue") >= 1,
            "server must send an interim `100 Continue` for `Expect: 100-continue`"
        );
        assert!(
            count_occurrences(&out, b"HTTP/1.1 200") >= 1,
            "the final 200 response must still follow the interim 100"
        );
        Ok(())
    }

    /// A Content-Length that is NOT a multiple of the read-buffer size must consume *exactly*
    /// `Content-Length` body bytes — not over-read into a following pipelined request. The old
    /// `read_exact([0; 1024])` loop demanded a full final 1024-byte chunk, swallowing bytes of the
    /// next request (and, against a live peer that keeps the socket open, blocking forever — the
    /// upload "hang" WiFiMaster/ops worked around with 1024/2048 payload padding).
    #[tokio::test]
    async fn fixed_body_is_not_over_read_into_a_pipelined_request() -> IoResult<()> {
        let mut raw = Vec::new();
        raw.extend_from_slice(b"POST /empty HTTP/1.1\r\nContent-Length: 1500\r\n\r\n");
        raw.extend_from_slice(&vec![b'x'; 1500]); // 1500 % 1024 == 476
        raw.extend_from_slice(b"GET /empty HTTP/1.1\r\n\r\n");
        let out = drive(raw).await?;
        assert_eq!(
            count_occurrences(&out, b"HTTP/1.1 200"),
            2,
            "both the POST and the pipelined GET must be answered; the body must stop at Content-Length"
        );
        Ok(())
    }

    // Minimal router stub: every request gets a 200, so the tests exercise body consumption
    // and the Expect handshake in `handle_socket` rather than the real routes.
    fn ok_router(_req: Request) -> Pin<Box<dyn Future<Output = Response> + Send>> {
        Box::pin(async { Response::res_200("") })
    }
}
