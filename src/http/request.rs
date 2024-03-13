use std::collections::HashMap;
use tokio::io::{AsyncRead, AsyncReadExt};

use anyhow::Result;

use crate::http::message::{Method, Version};

#[derive(Debug)]
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
            match length {
                Ok(length) => {
                    bodyvec = Vec::with_capacity(length);
                    body.read_exact(&mut bodyvec).await?;
                },
                Err(e) => return Err(e.into())
            }
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
