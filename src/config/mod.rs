use std::{
    collections::HashMap,
    fs::read_to_string,
    io::{Error, Result as IoResult, Write as _},
    path::Path,
    sync::OnceLock,
};

use chrono::Local;
use include_dir::{Dir, include_dir};
use log::{LevelFilter, info, trace};
use rand::RngExt as _;
use serde::Deserialize;
use serde_json::Value;
use tokio::runtime::{Builder, Runtime};

use crate::cmd::Cmd;

/* Static Values */
const CHUNK_SIZE: usize = 524_288; //512 Kilobytes x2
pub static DEF_ASSETS: Dir = include_dir!("assets");
pub static GARBAGE_DATA: OnceLock<Vec<u8>> = OnceLock::new();
pub const HEAD_ART: &str = r"
     _ _ _                                       _
    | (_) |__  _ __ ___  ___ _ __   ___  ___  __| |      _ __ ___
    | | | '_ \| '__/ _ \/ __| '_ \ / _ \/ _ \/ _` |_____| '__/ __|
    | | | |_) | | |  __/\__ \ |_) |  __/  __/ (_| |_____| |  \__ \
    |_|_|_.__/|_|  \___||___/ .__/ \___|\___|\__,_|     |_|  |___/
                            |_|
";
pub static ROUTES: OnceLock<HashMap<String, &str>> = OnceLock::new();
pub static SERVER_CONFIG: OnceLock<ServerConfig> = OnceLock::new();

#[derive(Deserialize, Debug)]
pub struct ServerConfig {
    pub assets_path: String,
    pub base_url: String,
    pub bind_address: String,
    pub enable_tls: bool,
    pub ipinfo_api_key: String,
    pub listen_port: u16,
    pub tls_cert_file: String,
    pub tls_key_file: String,
    pub worker_threads: Value,
}

trait SetIfSome<T> {
    fn set_if_some(&mut self, option: Option<T>);
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            assets_path: String::new(),
            base_url: "backend".to_owned(),
            bind_address: "0.0.0.0".to_owned(),
            enable_tls: false,
            ipinfo_api_key: String::new(),
            listen_port: 8080,
            tls_cert_file: String::new(),
            tls_key_file: String::new(),
            worker_threads: Value::from(1),
        }
    }
}

impl<T> SetIfSome<T> for T {
    fn set_if_some(&mut self, option: Option<T>) {
        if let Some(value) = option {
            *self = value;
        }
    }
}

impl<T> SetIfSome<T> for Option<T> {
    fn set_if_some(&mut self, option: Option<T>) {
        if let Some(value) = option {
            *self = Some(value);
        }
    }
}

fn check_assets_path(dir: &str) -> bool {
    let index_file = format!("{dir}/index.html");
    Path::new(&index_file).exists()
}

fn current_formatted_time() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string()
}

fn generate_routes(base_url: &str) {
    let mut routes = HashMap::new();
    let _ = routes.insert(format!("{base_url}/empty"), "empty");
    let _ = routes.insert(format!("{base_url}/garbage"), "garbage");
    let _ = routes.insert(format!("{base_url}/getIP"), "getIP");
    let _ = ROUTES.get_or_init(|| routes);
}

pub fn init_configs(cmd: Cmd) -> IoResult<()> {
    //init logger
    env_logger::builder()
        .format(|buf, rec| {
            let style = buf.default_level_style(rec.level());
            writeln!(
                buf,
                "[{} {style}{}{style:#} librespeed_rs] {}",
                current_formatted_time(),
                rec.level(),
                rec.args()
            )
        })
        .filter_level(LevelFilter::Info)
        .init();
    println!("{HEAD_ART}");
    //find server configs
    if let Some(config_path) = cmd.server_config_path.clone() {
        let loaded = open_config_file(&config_path).map_err(Error::other)?;
        initialize(loaded, cmd);
        info!("Configs initialized file : {config_path}");
    } else {
        // open config from current dir, otherwise set default config
        match open_config_file("configs.toml") {
            Ok(loaded) => {
                initialize(loaded, cmd);
                info!("Configs initialized file : configs.toml");
            }
            Err(err) => {
                initialize(ServerConfig::default(), cmd);
                info!("Configs initialized with defaults");
                trace!("Load config default path error : {err}");
            }
        }
    }
    Ok(())
}

pub fn init_runtime() -> IoResult<Runtime> {
    let configured_threads = SERVER_CONFIG
        .get()
        .ok_or_else(|| Error::other("Server config is not initialized"))?
        .worker_threads
        .clone();
    if configured_threads.is_string() && configured_threads == "auto" {
        Builder::new_multi_thread()
            .thread_name("librespeed-rs")
            .enable_io()
            .enable_time()
            .build()
    } else {
        let threads = configured_threads
            .as_u64()
            .and_then(|count| usize::try_from(count).ok())
            .unwrap_or(1);
        match threads {
            0 | 1 => Builder::new_current_thread().enable_io().enable_time().build(),
            _ => Builder::new_multi_thread()
                .thread_name("librespeed-rs")
                .worker_threads(threads)
                .enable_io()
                .enable_time()
                .build(),
        }
    }
}

fn initialize(mut config: ServerConfig, cmd: Cmd) {
    //server config
    config.base_url = validate_base_url_path(&config.base_url);
    config.bind_address.set_if_some(cmd.bind_address);
    config.listen_port.set_if_some(cmd.listen_port);
    config.base_url.set_if_some(cmd.base_url);
    config.ipinfo_api_key.set_if_some(cmd.ipinfo_api_key);
    config.assets_path.set_if_some(cmd.assets_path);
    config.enable_tls.set_if_some(cmd.enable_tls);
    config.tls_cert_file.set_if_some(cmd.tls_cert_file);
    config.tls_key_file.set_if_some(cmd.tls_key_file);
    generate_routes(&config.base_url);
    if config.assets_path.is_empty() {
        info!("Config default assets directory.");
    } else if check_assets_path(&config.assets_path) {
        info!("Config assets directory successfully.");
    } else {
        info!("Config assets directory failed !");
    }
    let _ = SERVER_CONFIG.get_or_init(|| config);
    //garbage data
    let chunk_len = format!("{CHUNK_SIZE:X}\r\n");
    let mut garbage = Vec::new();
    garbage.extend(chunk_len.as_bytes());
    let mut rng = rand::rng();
    garbage.extend((0..CHUNK_SIZE).map(|_| rng.random::<u8>()));
    garbage.extend(b"\r\n");
    let _ = GARBAGE_DATA.get_or_init(|| garbage);
}

fn open_config_file(path: &str) -> IoResult<ServerConfig> {
    let content = read_to_string(path).map_err(Error::other)?;
    toml::from_str(&content).map_err(Error::other)
}

fn validate_base_url_path(base_url: &str) -> String {
    let stripped = base_url.replace('/', "");
    if stripped.is_empty() {
        String::new()
    } else {
        format!("/{stripped}")
    }
}
