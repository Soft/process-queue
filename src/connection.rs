use anyhow::{anyhow, Result};
use bytes::{Buf, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

pub struct Connection {
    socket: UnixStream,
    buffer: BytesMut,
}

impl Connection {
    pub fn new(socket: UnixStream) -> Self {
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
        let buffer = BytesMut::with_capacity(page_size);
        Connection { socket, buffer }
    }

    pub async fn read_message<M>(&mut self) -> Result<Option<M>>
    where
        M: serde::de::DeserializeOwned,
    {
        loop {
            if let Some((message, end)) = parse_message(&self.buffer[..])? {
                self.buffer.advance(end);
                return Ok(Some(message));
            }

            if self.socket.read_buf(&mut self.buffer).await? == 0 {
                if self.buffer.is_empty() {
                    return Ok(None);
                } else {
                    return Err(anyhow!("disconnect"));
                }
            }
        }
    }

    pub async fn write_message<M>(&mut self, message: &M) -> Result<()>
    where
        M: serde::Serialize,
    {
        let buffer = serde_json::to_vec(message)?;
        self.socket.write_all(&buffer).await?;
        self.socket.write_u8(b'\0').await?;
        Ok(())
    }
}

fn parse_message<M>(buffer: &[u8]) -> Result<Option<(M, usize)>>
where
    M: serde::de::DeserializeOwned,
{
    if let Some((end, _)) = buffer.iter().enumerate().find(|(_, c)| **c == b'\0') {
        let message = serde_json::from_slice(&buffer[0..end])?;
        Ok(Some((message, end + 1)))
    } else {
        Ok(None)
    }
}
