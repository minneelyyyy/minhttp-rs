
use tokio::io::AsyncBufRead;

use anyhow::Result;

pub mod message;
pub mod reader;
pub mod request;
pub mod response;
pub mod stream;
pub mod writer;

pub trait Serialize {
    fn serialize(&self) -> Result<Vec<u8>>;
}

pub trait Deserialize<R: AsyncBufRead>
where
    Self: Sized
{
    async fn deserialize(reader: &mut R) -> Result<Self>;
}

pub trait AsyncWriteObj<T: Serialize> {
    async fn write_obj(&mut self, obj: &T) -> Result<()>;
}

pub trait AsyncReadObj<R: AsyncBufRead, T: Deserialize<R>> {
    async fn read_obj(&mut self) -> Result<T>;
}
