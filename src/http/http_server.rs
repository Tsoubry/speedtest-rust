use std::io::{Error, Result as IoResult};

use log::{info, trace};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter, split};
use tokio_rustls::TlsAcceptor;

use crate::{
    config::{ROUTES, SERVER_CONFIG},
    http::{
        Method, find_remote_ip_addr, get_chunk_count, request::handle_socket, response::Response,
        tcp_socket::TcpSocket, tls::setup_tls_acceptor,
    },
    ip::ip_info::IPInfo,
};

pub struct HttpServer {
    pub tcp_socket: TcpSocket,
    pub tls_acceptor: Option<TlsAcceptor>,
}

impl HttpServer {
    pub async fn handle_connection<R, W>(
        remote_addr: &str,
        buf_reader: &mut BufReader<R>,
        buf_writer: &mut BufWriter<W>,
    ) where
        R: AsyncReadExt + Unpin,
        W: AsyncWriteExt + Unpin, {
        handle_socket(remote_addr, buf_reader, buf_writer, |request| {
            Box::pin(async move {
                let matched_route = ROUTES.get().and_then(|routes| routes.get(request.path.trim()));
                if let Some(route) = matched_route {
                    match *route {
                        "empty" => Response::res_200(""),
                        "garbage" => {
                            let chunks = get_chunk_count(&request.query_params);
                            Response::res_200_garbage(chunks)
                        }
                        "getIP" => {
                            let get_isp = request
                                .query_params
                                .get("isp")
                                .is_some_and(|value| value.parse::<bool>().unwrap_or(false));
                            let ip_info = IPInfo::fetch_information(&request.remote_addr, get_isp).await;
                            Response::res_200_json(&ip_info)
                        }
                        _ => Response::res_404(),
                    }
                } else if matches!(request.method, Method::Get) {
                    Response::res_200_fs(request.path.trim())
                } else {
                    Response::res_404()
                }
            })
        })
        .await;
    }

    pub fn init() -> IoResult<Self> {
        let config = SERVER_CONFIG
            .get()
            .ok_or_else(|| Error::other("Server config is not initialized"))?;
        let tcp_socket = TcpSocket::make_listener(config)?;
        info!("Server started on {tcp_socket}");
        info!("Server base url : {}/", config.base_url);
        let mut tls_acceptor = None;
        if config.enable_tls {
            tls_acceptor = Some(setup_tls_acceptor(&config.tls_cert_file, &config.tls_key_file)?);
        }
        Ok(Self {
            tcp_socket,
            tls_acceptor,
        })
    }

    pub async fn listen(&mut self) {
        self.tcp_socket.spawn_signal_handler();
        let mut shutdown_rx = self.tcp_socket.shutdown_tx.subscribe();
        loop {
            let tcp_accept = self.tcp_socket.accept(&mut shutdown_rx).await;
            let acceptor = self.tls_acceptor.clone();

            match tcp_accept {
                Ok(Some((socket, _))) => {
                    drop(tokio::spawn(async move {
                        let Ok(remote_addr) = find_remote_ip_addr(&socket) else {
                            return;
                        };

                        if let Some(tls) = acceptor {
                            match tls.accept(socket).await {
                                Ok(tls_stream) => {
                                    let (socket_r, socket_w) = split(tls_stream);
                                    let mut buff_reader = BufReader::with_capacity(8 * 1024, socket_r);
                                    let mut buff_writer = BufWriter::with_capacity(8 * 1024, socket_w);
                                    Self::handle_connection(&remote_addr, &mut buff_reader, &mut buff_writer).await;
                                }
                                Err(err) => {
                                    trace!("Error tcp connection : {err}");
                                }
                            }
                        } else {
                            let mut plain_socket = socket;
                            let (socket_r, socket_w) = plain_socket.split();
                            let mut buff_reader = BufReader::with_capacity(8 * 1024, socket_r);
                            let mut buff_writer = BufWriter::with_capacity(8 * 1024, socket_w);
                            Self::handle_connection(&remote_addr, &mut buff_reader, &mut buff_writer).await;
                        }
                    }));
                }
                Ok(None) => {
                    info!("Shutdown signal received, stopping service ...");
                    info!("Bye 👋");
                    break;
                }
                Err(err) => {
                    trace!("Error tcp connection : {err}");
                }
            }
        }
    }
}
