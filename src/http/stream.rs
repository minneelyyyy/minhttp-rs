use tokio::io::{self, ReadHalf, WriteHalf, AsyncRead, AsyncWrite, BufReader};

use crate::http::reader::HttpReader;
use crate::http::writer::HttpWriter;

pub struct HttpStream<S: AsyncRead + AsyncWrite> {
    reader: HttpReader<BufReader<ReadHalf<S>>>,
    writer: HttpWriter<WriteHalf<S>>,
}

impl<S: AsyncRead + AsyncWrite> HttpStream<S> {
    pub fn new(stream: S) -> Self {
        let (reader, writer) = io::split(stream);
        let reader = BufReader::new(reader);

        let reader = HttpReader::new(reader);
        let writer = HttpWriter::new(writer);

        Self { reader, writer }
    }

    pub fn split(self) -> (HttpReader<BufReader<ReadHalf<S>>>, HttpWriter<WriteHalf<S>>) {
        (self.reader, self.writer)
    }

    #[allow(dead_code)]
    pub fn unsplit(reader: HttpReader<BufReader<ReadHalf<S>>>, writer: HttpWriter<WriteHalf<S>>) -> Self {
        Self { reader, writer }
    }
}
