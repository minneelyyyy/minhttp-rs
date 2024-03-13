use tokio::io::AsyncBufRead;

use crate::http::AsyncReadObj;
use crate::http::message::Message;

use anyhow::Result;

use super::Deserialize;

pub struct HttpReader<R: AsyncBufRead> {
    reader: R,
}

impl<R: AsyncBufRead> HttpReader<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }
}

impl<R: AsyncBufRead + Unpin> AsyncReadObj<R, Message> for HttpReader<R> {
    async fn read_obj(&mut self) -> Result<Message> {
        Message::deserialize(&mut self.reader).await
    }
}
