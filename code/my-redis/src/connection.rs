use tokio::net::TcpStream;
use mini_redis::{Frame, Result};
use mini_redis::frame::Error::Incomplete;
use bytes::{BytesMut, Buf};
use std::io::{Self, Cursor, BufWriter, AsyncWriteExt};

struct Connection {
    stream: BufWriter<TcpStream>,
    buffer: BytesMut,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Connection {
        Connection {
            stream: BufWriter::new(stream),
            buffer: BytesMut::with_capacity(4096),
        }
    }
    /// Read a frame from the connection.
    pub async fn read_frame(&mut self)
        -> Result<Option<Frame>>
    {
        loop {
            // Attempt to parse a frame if buffered enough
            if let Some(frame) = self.parse_frame()? {
                return Ok(Some(frame));
            }

            // End of stream
            if 0 == self.stream.read_buf(&mut self.buffer).await? {
                if self.buffer.is_empty() {
                    return Ok(None);
                } else {
                    return Err("connection reset by peer".into());
                }
            }
        }
    }

    fn parse_frame(&mut self)
        -> Result<Option<Frame>>
    {
        let mut buf = Cursor::new(&self.buffer[..]);

        // Check whether a full frame is available
        match Frame::check(&mut buf) {
            Ok(_) => {
                let len = buf.position() as usize;
                buf.set_position(0);

                // Parse the frame
                let frame = Frame::parse(&mut buf)?;
                
                // Discard the frame from the buffer
                self.buffer.advance(len);

                Ok(Some(frame))
            }
            Err(Incomplete) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Write a frame to the connection.
    pub async fn write_frame(&mut self, frame: &Frame)
        -> Result<()>
    {
        match frame {
            Frame::Simple(val) => {
                self.stream.write_u8(b'+').await?;
                self.stream.write_all(val.as_bytes()).await?;
                self.stream.write_all(b"\r\n").await?;
            }
            Frame::Error(val) => {
                self.stream.write_u8(b'-').await?;
                self.stream.write_all(val.as_bytes()).await?;
                self.stream.write_all(b"\r\n").await?;
            }
            Frame::Integer(val) => {
                self.stream.write_u8(b':').await?;
                self.write_decimal(*val).await?;
            }
            Frame::Null => {
                self.stream.write_all(b"$-1\r\n").await?;
            }
            Frame::Bulk(val) => {
                let len = val.len();

                self.stream.write_u8(b'$').await?;
                self.write_decimal(len as u64).await?;
                self.stream.write_all(val).await?;
                self.stream.write_all(b"\r\n").await?;
            }
            Frame:Array(_val) => unimplemented!(),
        }

        self.stream.flush().await;

        Ok(())
    }
}