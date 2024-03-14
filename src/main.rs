use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::fs::File;
use tokio::net;

use serde::Deserialize;

use anyhow::Result;

mod http;

use http::message::{Message, MessageParseError, Version};
use http::response::Response;
use http::request::Request;
use http::stream::HttpStream;
use http::{AsyncReadObj, AsyncWriteObj};

#[derive(Deserialize, Clone)]
struct HttpConfig {
    port: Option<u16>,
    address: Option<String>,
}

#[derive(Deserialize, Clone)]
struct HttpsConfig {
    port: Option<u16>,
    address: Option<String>,
    key: PathBuf,
    cert: PathBuf,
}

#[derive(Deserialize, Clone)]
struct Config {
    root: String,
    host: String,
    http: Option<HttpConfig>,
    https: Option<HttpsConfig>,
}

struct ServerInfo {
    root: String,
    host: String,
    port: u16,
}

impl ServerInfo {
    fn new(root: String, host: String, port: u16) -> Self {
        Self { root, host, port }
    }

    fn path(&self, pathstr: &str) -> String {
        format!("{}/{}", self.root, pathstr)
    }

    fn host_check(&self, host: &str) -> bool {
        *host == self.host || *host == format!("{}:{}", self.host, self.port)
    }
}

fn load_certs(path: &std::path::Path) -> std::io::Result<Vec<pki_types::CertificateDer<'static>>> {
    rustls_pemfile::certs(&mut std::io::BufReader::new(std::fs::File::open(path)?)).collect()
}

fn load_key(path: &std::path::Path) -> pki_types::PrivateKeyDer<'static> {
    rustls_pemfile::private_key(&mut std::io::BufReader::new(std::fs::File::open(path).unwrap())).unwrap().unwrap()
}

#[tokio::main]
async fn main() -> Result<()> {
    let config: Config = toml::from_str(&fs::read_to_string("minhttp.toml")?)?;
    let httphandle: Option<tokio::task::JoinHandle<Result<()>>>;
    let httpshandle: Option<tokio::task::JoinHandle<Result<()>>>;

    httphandle = config.http.clone().map(|http| {
        let config = config.clone();

        let address = http.address.unwrap_or("127.0.0.1".into());
        let port = http.port.unwrap_or(80);

        tokio::spawn(async move {
            let socket = net::TcpListener::bind(format!("{}:{}", address, port)).await?;

            loop {
                let (connection, _) = socket.accept().await?;
                
                let root = config.root.clone();
                let host = config.host.clone();

                tokio::spawn(async move {
                    match handle_connection(connection, ServerInfo::new(root, host, port)).await {
                        Ok(()) => (),
                        Err(e) => {
                            eprintln!("an error occured while handling request: {e}");
                        }
                    }
                });
            }
        })
    });

    httpshandle = config.https.clone().map(|https| {
        let config = config.clone();

        let address = https.address.unwrap_or("127.0.0.1".into());
        let port = https.port.unwrap_or(443);
        let certs = load_certs(&https.cert).unwrap();
        let key = load_key(&https.key);

        let rustlsconfig = tokio_rustls::rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err)).unwrap();

        tokio::spawn(async move {
            let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(rustlsconfig));
            let socket = net::TcpListener::bind(format!("{}:{}", address, port)).await?;

            loop {
                let (stream, _) = socket.accept().await?;
                let acceptor = acceptor.clone();

                let root = config.root.clone();
                let host = config.host.clone();

                tokio::spawn(async move {
                    let stream = acceptor.accept(stream).await.unwrap();

                    match handle_connection(stream, ServerInfo::new(root, host, port)).await {
                        Ok(()) => (),
                        Err(e) => {
                            eprintln!("an error occured while handling request: {e}");
                        }
                    }
                });
            }
        })
    });

    let _ = tokio::join!(
        httphandle.unwrap_or(tokio::spawn(async { Ok(()) })),
        httpshandle.unwrap_or(tokio::spawn(async { Ok(()) })),
    );

    Ok(())
}

async fn handle_connection<S: AsyncRead + AsyncWrite>(stream: S, config: ServerInfo) -> Result<()> {
    let http = HttpStream::new(stream);
    let (mut reader, mut writer) = http.split();

    loop {
        let msg: Message = match reader.read_obj().await {
            Ok(m) => m,
            Err(e) => {
                return e.downcast::<MessageParseError>().and_then(|msg_err| {
                    match msg_err {
                        MessageParseError::ConnectionClosed => Ok(()),
                        _ => Err(msg_err.into()),
                    }
                });
            }
        };

        match msg {
            Message::Request(req) => {
                println!("{} {}", req.method, req.resource);
                writer.write_obj(&create_response(req, &config).await?).await?;
            },

            Message::Response(_) => {
                writer.write_obj(&error(400, &config).await?).await?;
            },
        }
    }
}

fn get_filepath_from_code(code: u32) -> String {
    format!(".errors/{code}.html")
}

async fn error(code: u32, config: &ServerInfo) -> Result<Response> {
    let mut file = File::open(config.path(&get_filepath_from_code(code))).await?;
    Response::serve_file_with_code(Version::Http11, code, &mut file).await
}

async fn create_response(request: Request, config: &ServerInfo) -> Result<Response> {
    if request.headers.get("Host").filter(|h| config.host_check(*h)).is_none() {
        return error(400, config).await;
    }

    let md = match fs::metadata(config.path(&request.resource)) {
        Ok(m) => m,
        Err(_) => return error(404, config).await,
    };

    let path = if md.is_dir() {
        format!("{}/index.html", &request.resource)
    } else {
        request.resource
    };

    let mut file = File::open(&config.path(&path)).await?;

    Response::serve_file(Version::Http11, &mut file).await
}
