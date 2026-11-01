#![forbid(unsafe_code)]

use log::error;

use crate::{cmd::Cmd, http::http_server::HttpServer};

mod cmd;
mod config;
mod http;
mod ip;

fn main() {
    if let Err(e) = config::init_configs(Cmd::parse_args()) {
        error!("{e}");
        std::process::exit(1)
    }

    let runtime = config::init_runtime().unwrap_or_else(|e| {
        error!("{e}");
        std::process::exit(1)
    });

    runtime.block_on(async {
        match HttpServer::init().await {
            Ok(mut http_server) => http_server.listen().await,
            Err(e) => {
                error!("{e}");
                std::process::exit(1)
            }
        }
    });
}
