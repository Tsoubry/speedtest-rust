use std::path::Path;

use crate::http::get_index_file_content;

#[derive(Debug)]
pub struct Response {
    pub chunk_count: i32,
    pub data: Vec<u8>,
}

impl Response {
    pub fn res_200(content: &str) -> Self {
        let response_header = format!(
            "HTTP/1.1 200 OK\r\n\
            Content-Length: {}\r\n\
            Cache-Control: no-store, no-cache, must-revalidate, max-age=0, s-maxage=0\r\n\
            Cache-Control: post-check=0, pre-check=0\r\n\
            Pragma: no-cache\r\n\
            Access-Control-Allow-Origin: *\r\n\
            Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
            Access-Control-Allow-Headers: Content-Encoding, Content-Type\r\n\r\n{}",
            content.len(),
            content
        );
        Self {
            chunk_count: 0,
            data: response_header.as_bytes().to_vec(),
        }
    }

    pub fn res_200_fs(requested_file: &str) -> Self {
        let file_name = if requested_file == "/" {
            "/index.html"
        } else {
            requested_file
        };
        let content_type = match Path::new(file_name)
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("js") => "text/javascript",
            Some("html") => "text/html",
            Some("ico") => "image/vnd.microsoft.icon",
            Some("css") => "text/css",
            Some("svg") => "image/svg+xml",
            _ => return Self::res_404(),
        };
        let Some(file_content) = get_index_file_content(file_name) else {
            return Self::res_404();
        };
        let response_header = format!(
            "HTTP/1.1 200 OK\r\n\
                Content-Type: {content_type}\r\n\
                Content-Length: {}\r\n\
                Connection: keep-alive\r\n\
                Access-Control-Allow-Origin: *\r\n\
                Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\r\n",
            file_content.len()
        );
        let mut data = response_header.as_bytes().to_vec();
        data.extend(file_content);
        Self { chunk_count: 0, data }
    }

    pub fn res_200_garbage(chunk_count: i32) -> Self {
        let response_header = "HTTP/1.1 200 OK\r\n\
            Content-Description: File Transfer\r\n\
            Content-Type: application/octet-stream\r\n\
            Content-Disposition: attachment; filename=random.dat\r\n\
            Content-Transfer-Encoding: binary\r\n\
            Connection: keep-alive\r\n\
            Transfer-Encoding: chunked\r\n\
            Cache-Control: no-store, no-cache, must-revalidate, max-age=0, s-maxage=0\r\n\
            Cache-Control: post-check=0, pre-check=0\r\n\
            Pragma: no-cache\r\n\
            Access-Control-Allow-Origin: *\r\n\
            Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\r\n"
            .to_owned();
        Self {
            chunk_count,
            data: response_header.as_bytes().to_vec(),
        }
    }

    pub fn res_200_json(content: &str) -> Self {
        let response_header = format!(
            "HTTP/1.1 200 OK\r\n\
            Content-Length: {}\r\n\
            Content-Type: application/json; charset=utf-8\r\n\
            Cache-Control: no-store, no-cache, must-revalidate, max-age=0, s-maxage=0\r\n\
            Cache-Control: post-check=0, pre-check=0\r\n\
            Pragma: no-cache\r\n\
            Access-Control-Allow-Origin: *\r\n\
            Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
            Access-Control-Allow-Headers: Content-Encoding, Content-Type\r\n\r\n",
            content.len()
        );
        let mut data = response_header.as_bytes().to_vec();
        data.extend(content.as_bytes());
        Self { chunk_count: 0, data }
    }

    pub fn res_404() -> Self {
        let body = b"404 not found";
        let response_header = format!(
            "HTTP/1.1 404 Not Found\r\n\
            Content-Length: {}\r\n\
            Access-Control-Allow-Origin: *\r\n\
            Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\r\n",
            body.len()
        );
        let mut data = response_header.as_bytes().to_vec();
        data.extend(body);
        Self { chunk_count: 0, data }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{SERVER_CONFIG, ServerConfig};

    fn init_config() {
        let _ = SERVER_CONFIG.get_or_init(|| ServerConfig {
            assets_path: String::new(),
            base_url: "test".to_owned(),
            bind_address: "0.0.0.0".to_owned(),
            enable_tls: false,
            ipinfo_api_key: String::new(),
            listen_port: 8080,
            tls_cert_file: String::new(),
            tls_key_file: String::new(),
            worker_threads: serde_json::Value::from(1),
        });
    }

    #[test]
    fn fs_returns_correct_content_type_for_all_mime_types() {
        init_config();
        let cases = [
            ("/index.html", "200 OK", "Content-Type: text/html"),
            ("/", "200 OK", "Content-Type: text/html"),
            ("/speedtest.js", "200 OK", "Content-Type: text/javascript"),
            ("/servers_list.js", "200 OK", "Content-Type: text/javascript"),
            ("/favicon.ico", "200 OK", "Content-Type: image/vnd.microsoft.icon"),
            ("/unknown.xyz", "404 Not Found", ""),
            ("/nonexistent.html", "404 Not Found", ""),
        ];
        for (path, status, content_type) in &cases {
            let resp = Response::res_200_fs(path);
            let body = String::from_utf8_lossy(&resp.data);
            assert!(body.contains(status), "path={path}: expected status {status}");
            if !content_type.is_empty() {
                assert!(
                    body.contains(content_type),
                    "path={path}: expected Content-Type {content_type}"
                );
            }
        }
    }
}
