#![forbid(unsafe_code)]

mod cmd;
mod config;
mod http;
mod ip;

use std::process::exit;

use log::error;

use crate::{cmd::Cmd, http::http_server::HttpServer};

fn main() {
    if let Err(err) = config::init_configs(Cmd::parse_args()) {
        error!("{err}");
        exit(1);
    }

    let runtime = config::init_runtime().unwrap_or_else(|err| {
        error!("{err}");
        exit(1);
    });

    runtime.block_on(async {
        match HttpServer::init() {
            Ok(mut http_server) => http_server.listen().await,
            Err(err) => {
                error!("{err}");
                exit(1);
            }
        }
    });
}
