use std::{
    fs::File,
    io::{BufReader, Error, ErrorKind, Result as IoResult},
    sync::Arc,
};

use log::info;
use rustls_pemfile::{certs, private_key};
use tokio::net::TcpStream;
use tokio_rustls::{
    TlsAcceptor, TlsConnector,
    client::TlsStream,
    rustls::{
        ClientConfig, RootCertStore, ServerConfig,
        pki_types::{CertificateDer, PrivateKeyDer, ServerName},
    },
};

fn load_certs(path: &str) -> IoResult<Vec<CertificateDer<'static>>> {
    certs(&mut BufReader::new(open_file_buf(
        path,
        "Failed to load tls cert file",
    )?))
    .collect()
}

fn load_key(path: &str) -> IoResult<PrivateKeyDer<'static>> {
    private_key(&mut BufReader::new(open_file_buf(path, "Failed to load tls key file")?))?
        .ok_or_else(|| Error::other("Failed to load tls key file"))
}

fn open_file_buf(path: &str, err_msg: &str) -> IoResult<File> {
    File::open(path).map_err(|_| Error::other(err_msg.to_owned()))
}

/// TLS Configuration.
pub fn setup_tls_acceptor(cert_path: &str, key_path: &str) -> IoResult<TlsAcceptor> {
    let certs = load_certs(cert_path)?;
    let key = load_key(key_path)?;
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|err| Error::new(ErrorKind::InvalidInput, err))?;
    let acceptor = TlsAcceptor::from(Arc::new(config));
    info!("Server TLS successfully configured");
    Ok(acceptor)
}

pub async fn setup_tls_connector(domain: String, tcp_stream: TcpStream) -> IoResult<TlsStream<TcpStream>> {
    let mut root_cert_store = RootCertStore::empty();
    root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = ClientConfig::builder()
        .with_root_certificates(root_cert_store)
        .with_no_client_auth();
    let dns_name = ServerName::try_from(domain).map_err(|err| Error::new(ErrorKind::InvalidInput, err))?;
    let connector = TlsConnector::from(Arc::new(config));
    connector.connect(dns_name, tcp_stream).await
}
