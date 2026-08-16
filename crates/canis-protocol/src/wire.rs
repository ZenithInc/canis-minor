use std::{io, net::SocketAddrV4};

use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const STREAM_PROTOCOL_VERSION: u8 = 1;
pub const MAX_UDP_DATAGRAM: usize = 65_507;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamKind {
    Tcp,
    Udp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StreamRequest {
    pub kind: StreamKind,
    pub target: SocketAddrV4,
}

#[derive(Debug, Error)]
pub enum WireError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("unsupported stream protocol version {0}")]
    Version(u8),
    #[error("unsupported stream type {0}")]
    StreamKind(u8),
    #[error("unsupported address family {0}")]
    AddressFamily(u8),
    #[error("invalid zero target port")]
    ZeroPort,
    #[error("peer rejected stream: {0}")]
    Rejected(String),
    #[error("UDP datagram is too large: {0} bytes")]
    DatagramTooLarge(usize),
}

impl StreamRequest {
    pub async fn write_to<W>(&self, writer: &mut W) -> Result<(), WireError>
    where
        W: AsyncWrite + Unpin,
    {
        writer.write_u8(STREAM_PROTOCOL_VERSION).await?;
        writer
            .write_u8(match self.kind {
                StreamKind::Tcp => 1,
                StreamKind::Udp => 2,
            })
            .await?;
        writer.write_u8(4).await?;
        writer.write_all(&self.target.ip().octets()).await?;
        writer.write_u16(self.target.port()).await?;
        writer.flush().await?;
        Ok(())
    }

    pub async fn read_from<R>(reader: &mut R) -> Result<Self, WireError>
    where
        R: AsyncRead + Unpin,
    {
        let version = reader.read_u8().await?;
        if version != STREAM_PROTOCOL_VERSION {
            return Err(WireError::Version(version));
        }
        let kind = match reader.read_u8().await? {
            1 => StreamKind::Tcp,
            2 => StreamKind::Udp,
            other => return Err(WireError::StreamKind(other)),
        };
        let family = reader.read_u8().await?;
        if family != 4 {
            return Err(WireError::AddressFamily(family));
        }
        let mut ip = [0_u8; 4];
        reader.read_exact(&mut ip).await?;
        let port = reader.read_u16().await?;
        if port == 0 {
            return Err(WireError::ZeroPort);
        }
        Ok(Self {
            kind,
            target: SocketAddrV4::new(ip.into(), port),
        })
    }
}

pub async fn write_ok<W>(writer: &mut W) -> Result<(), WireError>
where
    W: AsyncWrite + Unpin,
{
    writer.write_u8(0).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn write_rejected<W>(writer: &mut W, message: &str) -> Result<(), WireError>
where
    W: AsyncWrite + Unpin,
{
    let bytes = message.as_bytes();
    let bytes = &bytes[..bytes.len().min(u8::MAX as usize)];
    writer.write_u8(1).await?;
    writer.write_u8(bytes.len() as u8).await?;
    writer.write_all(bytes).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn read_response<R>(reader: &mut R) -> Result<(), WireError>
where
    R: AsyncRead + Unpin,
{
    match reader.read_u8().await? {
        0 => Ok(()),
        1 => {
            let length = reader.read_u8().await? as usize;
            let mut message = vec![0_u8; length];
            reader.read_exact(&mut message).await?;
            Err(WireError::Rejected(
                String::from_utf8_lossy(&message).into_owned(),
            ))
        }
        other => Err(WireError::Rejected(format!(
            "invalid response status {other}"
        ))),
    }
}

pub async fn write_datagram<W>(writer: &mut W, datagram: &[u8]) -> Result<(), WireError>
where
    W: AsyncWrite + Unpin,
{
    if datagram.len() > MAX_UDP_DATAGRAM {
        return Err(WireError::DatagramTooLarge(datagram.len()));
    }
    writer.write_u16(datagram.len() as u16).await?;
    writer.write_all(datagram).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn read_datagram<R>(reader: &mut R) -> Result<Option<Vec<u8>>, WireError>
where
    R: AsyncRead + Unpin,
{
    let mut header = [0_u8; 2];
    match reader.read(&mut header[..1]).await {
        Ok(0) => return Ok(None),
        Ok(_) => {
            reader.read_exact(&mut header[1..]).await?;
        }
        Err(error) => return Err(error.into()),
    }
    let length = u16::from_be_bytes(header) as usize;
    let mut datagram = vec![0_u8; length];
    reader.read_exact(&mut datagram).await?;
    Ok(Some(datagram))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn address_round_trip() {
        let request = StreamRequest {
            kind: StreamKind::Udp,
            target: "203.0.113.7:5353".parse().unwrap(),
        };
        let (mut tx, mut rx) = tokio::io::duplex(64);
        let send = tokio::spawn(async move { request.write_to(&mut tx).await.unwrap() });
        let decoded = StreamRequest::read_from(&mut rx).await.unwrap();
        send.await.unwrap();
        assert_eq!(decoded, request);
    }

    #[tokio::test]
    async fn preserves_udp_boundaries_including_empty_datagrams() {
        let (mut tx, mut rx) = tokio::io::duplex(128);
        let send = tokio::spawn(async move {
            write_datagram(&mut tx, b"first").await.unwrap();
            write_datagram(&mut tx, b"").await.unwrap();
            write_datagram(&mut tx, b"third").await.unwrap();
        });
        assert_eq!(read_datagram(&mut rx).await.unwrap().unwrap(), b"first");
        assert_eq!(read_datagram(&mut rx).await.unwrap().unwrap(), b"");
        assert_eq!(read_datagram(&mut rx).await.unwrap().unwrap(), b"third");
        send.await.unwrap();
    }

    #[tokio::test]
    async fn rejects_wrong_stream_version() {
        let bytes = [99, 1, 4, 127, 0, 0, 1, 0, 80];
        let error = StreamRequest::read_from(&mut &bytes[..]).await.unwrap_err();
        assert!(matches!(error, WireError::Version(99)));
    }

    #[tokio::test]
    async fn rejects_a_truncated_datagram_header() {
        let error = read_datagram(&mut &[0_u8][..]).await.unwrap_err();
        assert!(matches!(error, WireError::Io(_)));
    }
}
