use crate::http::get_index_file_content;

#[derive(Debug)]
pub struct Response {
    pub data: Vec<u8>,
    pub chunk_count: i32,
}

impl Response {
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
        Response { data, chunk_count: 0 }
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
            .to_string();
        Response {
            data: response_header.as_bytes().to_vec(),
            chunk_count,
        }
    }

    pub fn res_200_fs(file_name: &str) -> Self {
        let file_name = if file_name == "/" { "/index.html" } else { file_name };
        if let Some(file_content) = get_index_file_content(file_name) {
            let content_type = match file_name {
                i if i.ends_with(".js") => "text/javascript",
                i if i.ends_with(".html") => "text/html",
                i if i.ends_with(".ico") => "image/vnd.microsoft.icon",
                i if i.ends_with(".css") => "text/css",
                i if i.ends_with(".svg") => "image/svg+xml",
                _ => "",
            };
            let data = match content_type {
                i if i == "text/javascript" || i == "text/html" || i == "text/css" || i == "image/svg+xml" => {
                    let content = String::from_utf8(file_content).unwrap();
                    format!(
                        "HTTP/1.1 200 OK\r\n\
                            Content-Type: {}\r\n\
                            Content-Length: {}\r\n\
                            Connection: keep-alive\r\n\
                            Access-Control-Allow-Origin: *\r\n\
                            Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\r\n{}",
                        content_type,
                        content.len(),
                        content
                    )
                    .as_bytes()
                    .to_vec()
                }
                "image/vnd.microsoft.icon" => {
                    let response_header = format!(
                        "HTTP/1.1 200 OK\r\n\
                            Content-Type: {}\r\n\
                            Content-Length: {}\r\n\
                            Connection: keep-alive\r\n\
                            Access-Control-Allow-Origin: *\r\n\
                            Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\r\n",
                        content_type,
                        file_content.len()
                    );
                    let mut data = response_header.as_bytes().to_vec();
                    data.extend(file_content);
                    data
                }
                _ => return Self::res_404(),
            };
            Response { data, chunk_count: 0 }
        } else {
            Self::res_404()
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
        Response { data, chunk_count: 0 }
    }

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
        Response {
            data: response_header.as_bytes().to_vec(),
            chunk_count: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{SERVER_CONFIG, ServerConfig};

    fn init_config() {
        SERVER_CONFIG.get_or_init(|| ServerConfig {
            bind_address: "0.0.0.0".to_string(),
            listen_port: 8080,
            worker_threads: serde_json::Value::from(1),
            base_url: "test".to_string(),
            ipinfo_api_key: "".to_string(),
            assets_path: "".to_string(),
            enable_tls: false,
            tls_cert_file: "".to_string(),
            tls_key_file: "".to_string(),
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
