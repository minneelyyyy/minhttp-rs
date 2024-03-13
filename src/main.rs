use std::collections::HashMap;
use std::fs;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::fs::File;
use tokio::net;

use serde::Deserialize;

use anyhow::Result;

mod http;

use http::message::{Message, Version};
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
    key: String,
    cert: String,
}

#[derive(Deserialize, Clone)]
struct ServerConfig {
    root: String,
    host: String,
    http: Option<HttpConfig>,
    https: Option<HttpsConfig>,
}

#[derive(Deserialize)]
struct Config {
    server: Vec<ServerConfig>,
}

struct ServerInfo {
    root: String,
    host: String,
    port: u16,
}

impl ServerInfo {
    fn path(&self, pathstr: &str) -> String {
        format!("{}/{}", self.root, pathstr)
    }

    fn host_check(&self, host: &str) -> bool {
        *host == self.host || *host == format!("{}:{}", self.host, self.port)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config: Config = toml::from_str(&fs::read_to_string("minhttp.toml")?)?;

    for server in config.server {
        let handle = tokio::spawn(async move {
            if server.http.is_none() {
                eprintln!("No HTTP defined!");
                return;
            }

            let http = server.http.clone().unwrap();

            let listen = net::TcpListener::bind(format!("{}:{}",
                http.address.unwrap_or("127.0.0.1".into()), http.port.unwrap_or(80))
            ).await.expect("failed to open listen socket");

            loop {
                let (socket, _) = listen.accept().await.expect("failed to accept connection");

                let server = server.clone();

                tokio::spawn(async move {
                    match handle_connection(socket, ServerInfo {
                        root: server.root,
                        host: server.host,
                        port: server.http.unwrap().port.unwrap_or(80)
                    }).await {
                        Ok(()) => (),
                        Err(e) => {
                            eprintln!("An error occured while handling request: {e}");
                        }
                    }
                });
            }
        });

        let _ = handle.await;
    }

    Ok(())
}

async fn handle_connection<S: AsyncRead + AsyncWrite>(stream: S, config: ServerInfo) -> Result<()> {
    let http = HttpStream::new(stream);
    let (mut reader, mut writer) = http.split();

    loop {
        let msg: Message = reader.read_obj().await?;

        match msg {
            Message::Request(req) => {
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

    let mut file = File::open(&config.path(&path)).await.unwrap();

    Response::serve_file(Version::Http11, &mut file).await
}
