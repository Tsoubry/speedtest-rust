use std::net::{IpAddr, Ipv4Addr};

use log::warn;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{config::SERVER_CONFIG, http::http_client::HttpClient};

#[allow(
    non_snake_case,
    reason = "field names must match the JSON keys expected by the frontend"
)]
#[derive(Serialize, Deserialize)]
pub struct IPInfo {
    pub processedString: String,
    pub rawIspInfo: RawISPInfo,
}

#[derive(Serialize, Deserialize)]
pub struct RawISPInfo {
    pub city: String,
    pub country: String,
    pub hostname: String,
    pub ip: String,
    #[serde(rename = "loc")]
    pub location: String,
    #[serde(rename = "org")]
    pub organization: String,
    pub postal: String,
    pub readme: Option<String>,
    pub region: String,
    pub timezone: String,
}

impl Default for IPInfo {
    fn default() -> Self {
        Self {
            processedString: String::new(),
            rawIspInfo: RawISPInfo {
                city: String::new(),
                country: String::new(),
                hostname: String::new(),
                ip: String::new(),
                location: String::new(),
                organization: String::new(),
                postal: String::new(),
                readme: None,
                region: String::new(),
                timezone: String::new(),
            },
        }
    }
}

impl IPInfo {
    pub async fn fetch_information(raw_ip: &str, get_isp: bool) -> String {
        let mut ip_info_model = Self::default();

        if !get_isp {
            raw_ip.clone_into(&mut ip_info_model.processedString);
            let json_string = json!(ip_info_model);
            return json_string.to_string();
        }

        //local ip, no need to get more information
        if let Some(local_ip_info) = Self::get_local_or_private_info(raw_ip) {
            ip_info_model.processedString = format!("{raw_ip} - {local_ip_info}");
            let json_string = json!(ip_info_model);
            return json_string.to_string();
        }

        //get isp info from api
        if let Some(isp_info) = Self::get_isp_info_from_api(raw_ip).await {
            ip_info_model.processedString = isp_info;
            let json_string = json!(ip_info_model);
            return json_string.to_string();
        }

        //failed to get isp, send only ip
        raw_ip.clone_into(&mut ip_info_model.processedString);
        json!(ip_info_model).to_string()
    }

    async fn get_isp_info_from_api(ip: &str) -> Option<String> {
        let config = SERVER_CONFIG.get()?;
        let ip_info_token = config.ipinfo_api_key.clone();
        if ip_info_token.is_empty() {
            return None;
        }
        if let Ok(mut client) = HttpClient::open("https://ipinfo.io").await {
            let request = format!(
                "GET /{ip}/json?token={ip_info_token} HTTP/1.1\r\n\
                Host: ipinfo.io\r\n\r\n"
            );
            if let Ok(Some(res_body)) = client.send_request_json(request.as_bytes()).await {
                let isp = if let Some(org) = res_body.get("org") {
                    org.as_str()?
                } else {
                    res_body.get("asn")?.get("name")?.as_str()?
                };
                let country = res_body.get("country")?.as_str()?;
                Some(format!("{ip} - {isp}, {country}"))
            } else {
                None
            }
        } else {
            warn!("Http client error.");
            None
        }
    }

    fn get_local_or_private_info(ip: &str) -> Option<String> {
        if "::1" == ip {
            return Some("localhost IPv6 access".to_owned());
        }
        if ip.starts_with("fe80:") {
            return Some("link-local IPv6 access".to_owned());
        }
        if Self::is_ula_ipv6(ip) {
            return Some("ULA IPv6 access".to_owned());
        }
        if ip.starts_with("127.") {
            return Some("localhost IPv4 access".to_owned());
        }
        if Self::is_private_ipv4(ip) {
            return Some("private IPv4 access".to_owned());
        }
        if ip.starts_with("169.254.") {
            return Some("link-local IPv4 access".to_owned());
        }
        None
    }

    fn is_private_ipv4(ip: &str) -> bool {
        ip.parse::<Ipv4Addr>().is_ok_and(|ip_addr| ip_addr.is_private())
    }

    fn is_ula_ipv6(ip: &str) -> bool {
        match ip.parse::<IpAddr>() {
            Ok(IpAddr::V6(v6_addr)) => v6_addr.octets().first().is_some_and(|octet| octet & 0xc0 == 0xc0),
            Ok(IpAddr::V4(_)) | Err(_) => false,
        }
    }
}
