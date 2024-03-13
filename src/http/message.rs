use std::fmt::{self, Display};
use std::error::Error;
use std::collections::HashMap;
use std::iter::Iterator;
use tokio::io::{AsyncBufRead, AsyncBufReadExt};

use anyhow::Result;

use crate::http::Deserialize;
use crate::http::{request::Request, response::Response};

#[derive(Debug)]
pub enum MessageParseError {
    RequestLineRead,
    RequestLineParse,
    Header,
}

impl Display for MessageParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Self::Header => "failed to parse header",
            Self::RequestLineRead => "failed to read in a request line",
            Self::RequestLineParse => "failed to parse request line",
        })
    }
}

impl Error for MessageParseError {}

#[derive(Debug)]
pub enum Method {
    Get,
    Post,
}

impl Method {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "GET" => Some(Self::Get),
            "POST" => Some(Self::Post),
            _ => None
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
        }
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

#[derive(Debug)]
pub enum Version {
    Http11,
    #[allow(dead_code)]
    Http2,
    #[allow(dead_code)]
    Http3,
}

impl Version {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "HTTP/1.1" => Some(Self::Http11),
            "HTTP/2" => Some(Self::Http2),
            "HTTP/3" => Some(Self::Http3),
            _ => None
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            Self::Http11 => "HTTP/1.1",
            Self::Http2 => "HTTP/2",
            Self::Http3 => "HTTP/3",
        }
    }
}

#[derive(Debug)]
pub enum Message {
    Request(Request),
    Response(Response),
}

impl Message {
    async fn parse<R: AsyncBufRead + Unpin>(request_line: &str, headers: HashMap<String, String>, body: &mut R) -> Result<Self> {
        match request_line.splitn(3, ' ').collect::<Vec<&str>>().as_slice() {
            ["HTTP/1.1", errno, errstr] => Ok(Message::Response(Response::new(Version::Http11, errno.parse()?, errstr, headers, body).await?)),
            [method, resource, "HTTP/1.1"] => Ok(Message::Request(Request::new(Method::from_str(method).unwrap(), resource, Version::Http11, headers, body).await?)),
            _ => Err(MessageParseError::RequestLineParse.into()),
        }
    }
}

impl<R: AsyncBufRead + Unpin> Deserialize<R> for Message {
    async fn deserialize(reader: &mut R) -> Result<Self> {
        let mut lines = reader.lines();
        let mut request_line = String::new();

        while let Some(line) = lines.next_line().await? {
            if !line.is_empty() {
                request_line = line;
                break;
            }
        }

        if request_line.is_empty() {
            return Err(MessageParseError::RequestLineRead.into());
        }

        let mut headers: HashMap<String, String> = HashMap::new();

        while let Some(line) = lines.next_line().await? {
            if line.is_empty() {
                break;
            }

            let (left, right) = line.split_once(": ").ok_or(MessageParseError::Header)?;

            headers.insert(left.into(), right.into());
        }

        Message::parse(&request_line, headers, lines.get_mut()).await
    }
}
