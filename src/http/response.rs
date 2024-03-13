use std::collections::HashMap;
use std::fmt::Write as _;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::fs::File;

use anyhow::Result;

use crate::http::message::Version;
use crate::http::Serialize;

#[derive(Debug)]
pub struct Response {
    pub version: Version,
    pub code: u32,
    pub message: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Response {
    pub async fn new<R>(version: Version, errno: u32, errstr: &str, headers: HashMap<String, String>, body: &mut R) -> Result<Self>
    where
        R: AsyncRead + Unpin
    {
        let mut bodyvec = vec![];

        if let Some(length) = headers.get("Content-Length").map(|l| l.parse::<usize>()) {
            match length {
                Ok(length) => {
                    bodyvec = vec![0u8; length];
                    body.read_exact(&mut bodyvec).await?;
                },
                Err(e) => return Err(e.into())
            }
        }

        Ok(Self {
            version,
            code: errno,
            message: errstr.into(),
            headers,
            body: bodyvec,
        })
    }

    pub fn message(code: u32) -> Option<&'static str> {
        match code {
            100 => Some("Continue"),
            101 => Some("Switching Protocols"),
            102 => Some("Processing"),
            103 => Some("Early Hints"),

            200 => Some("OK"),
            201 => Some("Created"),
            202 => Some("Accepted"),
            203 => Some("Non-Authorative Information"),
            204 => Some("No Content"),
            205 => Some("Reset Content"),
            206 => Some("Partial Content"),
            207 => Some("Multi-Status"),
            208 => Some("Already Reporting"),
            226 => Some("IM Used"),

            300 => Some("Multiple Choices"),
            301 => Some("Moved Permanently"),
            302 => Some("Found"),
            303 => Some("See Other"),
            304 => Some("Not Modified"),
            307 => Some("Temporary Redirect"),
            308 => Some("Permanent Redirect"),

            400 => Some("Bad Request"),
            401 => Some("Unauthorized"),
            403 => Some("Forbidden"),
            404 => Some("Not Found"),
            405 => Some("Method Not Allowed"),
            406 => Some("Not Acceptable"),
            407 => Some("Proxy Authentication Required"),
            408 => Some("Request Timeout"),
            409 => Some("Conflict"),
            410 => Some("Gone"),
            411 => Some("Length Required"),
            412 => Some("Precondition Failed"),
            413 => Some("Payload Too Large"),
            414 => Some("URI Too Long"),
            415 => Some("Unsupported Media Type"),
            416 => Some("Range Not Satisfyable"),
            417 => Some("Expectation Failed"),
            418 => Some("I'm a teapot"),
            421 => Some("Misdirect Request"),
            422 => Some("Unprocessable Content"),
            423 => Some("Locked"),
            424 => Some("Failed Dependency"),
            426 => Some("Upgrade Required"),
            428 => Some("Precondition Required"),
            429 => Some("Too Many Requests"),
            431 => Some("Request Header Fields Too Large"),
            451 => Some("Unavailable for Legal Reasons"),

            500 => Some("Internal Server Error"),
            501 => Some("Not Implemented"),
            502 => Some("Bad Gateway"),
            503 => Some("Service Unavailable"),
            504 => Some("Gateway Timeout"),
            505 => Some("HTTP Version Not Supported"),
            506 => Some("Variant Also Negotiates"),
            507 => Some("Insufficient Storage"),
            508 => Some("Loop Detected"),
            510 => Some("Not Extended"),
            511 => Some("Network Authentication Required"),

            _ => None
        }
    }

    pub async fn serve_file_with_code(version: Version, code: u32, file: &mut File) -> Result<Self> {
        let headers = HashMap::from([
            ("Content-Length".into(), file.metadata().await?.len().to_string())
        ]);

        Self::new(version, code, Self::message(code).unwrap_or("Unknown Code"), headers, file).await
    }

    pub async fn serve_file(version: Version, file: &mut File) -> Result<Self> {
        Self::serve_file_with_code(version, 200, file).await
    }
}

impl Serialize for Response {
    fn serialize(&self) -> Result<Vec<u8>> {
        let mut out = String::new();

        write!(out, "{} {} {}\r\n", self.version.to_str(), self.code.to_string(), self.message)?;

        for header in &self.headers {
            write!(out, "{}: {}\r\n", header.0, header.1)?;
        }

        write!(out, "\r\n")?;

        let mut out: Vec<u8> = out.bytes().collect();
        let mut data = self.body.clone();

        out.append(&mut data);

        Ok(out)
    }
}
