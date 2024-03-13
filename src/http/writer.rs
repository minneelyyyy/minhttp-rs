use tokio::io::{AsyncWrite, AsyncWriteExt};

use anyhow::Result;

use super::AsyncWriteObj;
use super::Serialize;

pub struct HttpWriter<W: AsyncWrite> {
    writer: W,
}

impl<W: AsyncWrite> HttpWriter<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }
}

impl<W: AsyncWrite + Unpin, T: Serialize> AsyncWriteObj<T> for HttpWriter<W> {
    async fn write_obj(&mut self, obj: &T) -> Result<()> {
        let raw = obj.serialize()?;
        self.writer.write(&raw).await?;

        Ok(())
    }
}
