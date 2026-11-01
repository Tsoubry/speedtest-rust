pub mod http_client;
pub mod http_server;
pub mod request;
pub mod response;
mod tcp_socket;
pub mod tls;

use std::{
    collections::HashMap,
    fs::File,
    io::{Read as _, Result as IoResult},
};

use tokio::net::TcpStream;

use crate::config::{DEF_ASSETS, SERVER_CONFIG};

#[derive(Debug)]
pub enum Method {
    Get,
    Post,
}

pub trait MethodStr {
    fn to_method(&self) -> Method;
}

impl MethodStr for str {
    fn to_method(&self) -> Method {
        match self {
            "POST" => Method::Post,
            _ => Method::Get,
        }
    }
}

pub fn find_remote_ip_addr(conn: &TcpStream) -> IoResult<String> {
    let client_addr = conn.peer_addr()?;
    Ok(client_addr.ip().to_string().replace("::ffff:", ""))
}

fn generate_server_endpoint(configured_base_url: &str) -> Vec<u8> {
    let base_url = configured_base_url
        .strip_prefix('/')
        .map_or_else(String::new, |stripped| format!("{stripped}/"));
    let endpoint = format!(
        r#"function get_servers() {{
        return [
            {{
                name : "Simple Server",
                server : window.location.origin,
                dlURL: "{base_url}garbage",
                ulURL: "{base_url}empty",
                pingURL: "{base_url}empty",
                getIpURL: "{base_url}getIP"
            }}
        ]
    }}"#
    );
    Vec::from(endpoint.as_bytes())
}

pub fn get_chunk_count(query_params: &HashMap<String, String>) -> i32 {
    let chunks = query_params
        .get("ckSize")
        .and_then(|ck_size| ck_size.parse::<i32>().ok())
        .map_or(4, |parsed_ck_size| parsed_ck_size.min(1024));
    chunks * 2
}

pub fn get_index_file_content(file_name: &str) -> Option<Vec<u8>> {
    let config = SERVER_CONFIG.get()?;
    if config.assets_path.is_empty() {
        if file_name.contains("servers_list.js") {
            Some(generate_server_endpoint(&config.base_url))
        } else {
            let relative_name = file_name.strip_prefix('/').unwrap_or(file_name);
            let file = DEF_ASSETS.get_file(relative_name)?;
            Some(Vec::from(file.contents()))
        }
    } else {
        let file_path = format!("{}{}", config.assets_path, file_name);
        let mut file = File::open(file_path).ok()?;
        let mut file_bytes = Vec::new();
        let _ = file.read_to_end(&mut file_bytes).ok()?;
        Some(file_bytes)
    }
}
