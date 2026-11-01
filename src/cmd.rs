use clap::{Arg, Command, value_parser};

#[derive(Debug)]
pub struct Cmd {
    pub server_config_path: Option<String>,
    pub bind_address: Option<String>,
    pub listen_port: Option<u16>,
    pub base_url: Option<String>,
    pub ipinfo_api_key: Option<String>,
    pub assets_path: Option<String>,
    pub enable_tls: Option<bool>,
    pub tls_cert_file: Option<String>,
    pub tls_key_file: Option<String>,
}

const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const PKG_NAME: &str = env!("CARGO_PKG_NAME");
const PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const PKG_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

impl Cmd {
    pub fn parse_args() -> Self {
        let args = Command::new(PKG_NAME)
            .version(PKG_VERSION)
            .author(PKG_AUTHORS)
            .about(PKG_DESCRIPTION)
            .arg(Arg::new("server-config-path").short('c').long("config"))
            .arg(
                Arg::new("bind-address")
                    .short('b')
                    .long("bind-address")
                    .help("Bind IP address")
                    .value_parser(value_parser!(String)),
            )
            .arg(
                Arg::new("listen-port")
                    .short('p')
                    .long("listen-port")
                    .help("Listening port")
                    .value_parser(value_parser!(u16)),
            )
            .arg(
                Arg::new("base-url")
                    .long("base-url")
                    .help("Specify base api url /{base_url}/routes")
                    .value_parser(value_parser!(String)),
            )
            .arg(
                Arg::new("ipinfo-api-key")
                    .long("ipinfo-api-key")
                    .help("Specify the ipinfo API key")
                    .value_parser(value_parser!(String)),
            )
            .arg(
                Arg::new("assets-path")
                    .long("assets-path")
                    .help("Specify the directory of speedtest web frontend")
                    .value_parser(value_parser!(String)),
            )
            .arg(
                Arg::new("enable-tls")
                    .long("enable-tls")
                    .help("Enable and use TLS server")
                    .value_parser(value_parser!(bool)),
            )
            .arg(
                Arg::new("tls-cert-file")
                    .long("tls-cert-file")
                    .help("Specify the certificate file path")
                    .value_parser(value_parser!(String)),
            )
            .arg(
                Arg::new("tls-key-file")
                    .long("tls-key-file")
                    .help("Specify the key file path")
                    .value_parser(value_parser!(String)),
            )
            .get_matches();
        let server_config_path: Option<String> = args.get_one::<String>("server-config-path").map(|s| s.to_owned());
        let bind_address: Option<String> = args.get_one::<String>("bind-address").map(|s| s.to_owned());
        let listen_port: Option<u16> = args.get_one::<u16>("listen-port").map(|s| s.to_owned());
        let base_url: Option<String> = args.get_one::<String>("base-url").map(|s| s.to_owned());
        let ipinfo_api_key: Option<String> = args.get_one::<String>("ipinfo-api-key").map(|s| s.to_owned());
        let assets_path: Option<String> = args.get_one::<String>("assets-path").map(|s| s.to_owned());
        let enable_tls: Option<bool> = args.get_one::<bool>("enable-tls").map(|s| s.to_owned());
        let tls_cert_file: Option<String> = args.get_one::<String>("tls-cert-file").map(|s| s.to_owned());
        let tls_key_file: Option<String> = args.get_one::<String>("tls-key-file").map(|s| s.to_owned());
        Cmd {
            server_config_path,
            bind_address,
            listen_port,
            base_url,
            ipinfo_api_key,
            assets_path,
            enable_tls,
            tls_cert_file,
            tls_key_file,
        }
    }
}
