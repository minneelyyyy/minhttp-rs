use std::fmt::{self, Display};
use std::error::Error;
use std::collections::HashMap;
use std::iter::Iterator;
use tokio::io::{AsyncBufRead, AsyncBufReadExt};

use anyhow::Result;

use crate::http::Deserialize;
use crate::http::Serialize;
use crate::http::{request::Request, response::Response};

#[derive(Debug)]
pub enum MessageParseError {
    ConnectionClosed,
    RequestLineParse,
    Header,
}

impl Display for MessageParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Self::Header => "failed to parse header",
            Self::ConnectionClosed => "the connection was closed",
            Self::RequestLineParse => "failed to parse request line",
        })
    }
}

impl Error for MessageParseError {}

pub enum Method {
    Get,
    Head,
    Post,
    Put,
    Delete,
    Connect,
    Options,
    Trace,
    Patch,
}

#[derive(Debug)]
pub enum MethodParseError {
    InvalidMethod,
}

impl Display for MethodParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Self::InvalidMethod => "the method supplied does not exist",
        })
    }
}

impl Error for MethodParseError {}

impl std::str::FromStr for Method {
    type Err = MethodParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GET" => Ok(Self::Get),
            "HEAD" => Ok(Self::Head),
            "POST" => Ok(Self::Post),
            "PUT" => Ok(Self::Put),
            "DELETE" => Ok(Self::Delete),
            "CONNECT" => Ok(Self::Connect),
            "OPTIONS" => Ok(Self::Options),
            "TRACE" => Ok(Self::Trace),
            "PATCH" => Ok(Self::Patch),
            _ => Err(MethodParseError::InvalidMethod),
        }
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let m = match self {
            Self::Get => "GET",
            Self::Head => "HEAD",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
            Self::Connect => "CONNECT",
            Self::Options => "OPTIONS",
            Self::Trace => "TRACE",
            Self::Patch => "PATCH",
        };

        write!(f, "{}", m)
    }
}

pub enum Version {
    Http11,
    Http2,
    Http3,
}

#[derive(Debug)]
pub enum VersionParseError {
    InvalidVersion,
}

impl fmt::Display for VersionParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let v = match self {
            Self::InvalidVersion => "the version supplied does not exist",
        };

        write!(f, "{}", v)
    }
}

impl Error for VersionParseError {}

impl std::str::FromStr for Version {
    type Err = VersionParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "HTTP/1.1" => Ok(Self::Http11),
            "HTTP/2" => Ok(Self::Http2),
            "HTTP/3" => Ok(Self::Http3),
            _ => Err(VersionParseError::InvalidVersion.into())
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Self::Http11 => "HTTP/1.1",
            Self::Http2 => "HTTP/2",
            Self::Http3 => "HTTP/3",
        })
    }
}

pub enum Message {
    Request(Request),
    Response(Response),
}

impl Message {
    async fn parse<R: AsyncBufRead + Unpin>(request_line: &str, headers: HashMap<String, String>, body: &mut R) -> Result<Self> {
        let parts = request_line.splitn(3, ' ').collect::<Vec<&str>>();

        if parts.len() != 3 {
            return Err(MessageParseError::RequestLineParse.into());
        }

        if let Ok(method) = parts[0].parse::<Method>() {
            let (method, resource, version) = (method, parts[1], parts[2]);
            Ok(Request::new(method, resource, version.parse()?, headers, body).await?.into())
        } else if let Ok(version) = parts[0].parse::<Version>() {
            let (version, code, message) = (version, parts[1], parts[2]);
            Ok(Response::new(version, code.parse()?, message, headers, body).await?.into())
        } else {
            Err(MessageParseError::RequestLineParse.into())
        }
    }
}

impl<R: AsyncBufRead + Unpin> Deserialize<R> for Message {
    async fn deserialize(reader: &mut R) -> Result<Self> {
        let mut lines = reader.lines();

        let request_line = match lines.next_line().await? {
            Some(r) => r,
            None => return Err(MessageParseError::ConnectionClosed.into()),
        };

        let mut headers: HashMap<String, String> = HashMap::new();

        while let Some(line) = lines.next_line().await?.filter(|l| !l.is_empty()) {
            let (left, right) = line.split_once(": ").ok_or(MessageParseError::Header)?;
            headers.insert(left.into(), right.into());
        }

        Message::parse(&request_line, headers, lines.get_mut()).await
    }
}

impl Serialize for Message {
    fn serialize(&self) -> Result<Vec<u8>> {
        match self {
            Self::Request(req) => req.serialize(),
            Self::Response(res) => res.serialize(),
        }
    }
}

impl From<Request> for Message {
    fn from(value: Request) -> Self {
        Self::Request(value)
    }
}

impl From<Response> for Message {
    fn from(value: Response) -> Self {
        Self::Response(value)
    }
}
