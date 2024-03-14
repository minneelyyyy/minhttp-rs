use std::collections::HashMap;
use std::fmt::Write;
use tokio::io::{AsyncRead, AsyncReadExt};

use anyhow::Result;

use crate::http::message::{Method, Version};
use crate::http::Serialize;

pub struct Request {
    pub method: Method,
    pub resource: String,
    pub version: Version,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Request {
    pub async fn new<R>(method: Method, resource: &str, version: Version, headers: HashMap<String, String>, body: &mut R) -> Result<Self>
    where
        R: AsyncRead + Unpin
    {
        let mut bodyvec = vec![];

        if let Some(length) = headers.get("Content-Length").map(|l| l.parse::<usize>()) {
            let length = length?;

            bodyvec = vec![0u8; length];
            body.read_exact(&mut bodyvec).await?;
        }

        Ok(Self {
            method,
            resource: resource.to_string(),
            version,
            headers,
            body: bodyvec,
        })
    }
}

impl Serialize for Request {
    fn serialize(&self) -> Result<Vec<u8>> {
        let mut out = String::new();

        write!(out, "{} {} {}\r\n", self.method, self.resource, self.version)?;

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
