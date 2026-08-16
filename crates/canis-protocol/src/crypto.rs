use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, anyhow, ensure};
use snow::{Builder, HandshakeState, TransportState, params::NoiseParams};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, DuplexStream, ReadHalf, WriteHalf},
    task::JoinHandle,
};

use crate::PROTOCOL_VERSION;

const MAGIC: &[u8; 4] = b"CMNR";
const NOISE_PATTERN: &str = "Noise_NNpsk0_25519_ChaChaPoly_BLAKE2s";
const PROLOGUE: &[u8] = b"canis-minor tunnel v1";
const MAX_HANDSHAKE_RECORD: usize = 1024;
const MAX_PLAINTEXT_RECORD: usize = 16 * 1024;
const AUTH_TAG_LEN: usize = 16;

pub async fn client_handshake<T>(io: &mut T, psk: &[u8; 32]) -> Result<TransportState>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    write_preface(io).await?;
    read_preface(io).await?;

    let mut state = noise_builder(psk)?.build_initiator()?;
    let message = write_handshake_message(&mut state)?;
    write_record(io, &message).await?;
    let response = read_record(io, MAX_HANDSHAKE_RECORD).await?;
    read_handshake_message(&mut state, &response)?;
    ensure!(
        state.is_handshake_finished(),
        "Noise handshake did not finish"
    );
    state
        .into_transport_mode()
        .context("failed to enter Noise transport mode")
}

pub async fn server_handshake<T>(io: &mut T, psk: &[u8; 32]) -> Result<TransportState>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    read_preface(io).await?;
    write_preface(io).await?;

    let mut state = noise_builder(psk)?.build_responder()?;
    let request = read_record(io, MAX_HANDSHAKE_RECORD).await?;
    read_handshake_message(&mut state, &request).context("Noise authentication failed")?;
    let response = write_handshake_message(&mut state)?;
    write_record(io, &response).await?;
    ensure!(
        state.is_handshake_finished(),
        "Noise handshake did not finish"
    );
    state
        .into_transport_mode()
        .context("failed to enter Noise transport mode")
}

pub fn spawn_encrypted_transport<T>(
    io: T,
    state: TransportState,
) -> (DuplexStream, JoinHandle<Result<()>>)
where
    T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (application, pump) = tokio::io::duplex(256 * 1024);
    let state = Arc::new(Mutex::new(state));
    let (network_reader, network_writer) = tokio::io::split(io);
    let (plain_reader, plain_writer) = tokio::io::split(pump);

    let outgoing_state = Arc::clone(&state);
    let outgoing = pump_outgoing(plain_reader, network_writer, outgoing_state);
    let incoming = pump_incoming(network_reader, plain_writer, state);
    let task = tokio::spawn(async move {
        tokio::try_join!(outgoing, incoming)?;
        Ok(())
    });
    (application, task)
}

fn noise_builder(psk: &[u8; 32]) -> Result<Builder<'_>> {
    let params: NoiseParams = NOISE_PATTERN.parse().context("invalid Noise pattern")?;
    Builder::new(params)
        .prologue(PROLOGUE)?
        .psk(0, psk)
        .context("failed to configure Noise PSK")
}

fn write_handshake_message(state: &mut HandshakeState) -> Result<Vec<u8>> {
    let mut output = vec![0_u8; MAX_HANDSHAKE_RECORD];
    let size = state
        .write_message(&[], &mut output)
        .context("failed to write Noise handshake message")?;
    output.truncate(size);
    Ok(output)
}

fn read_handshake_message(state: &mut HandshakeState, message: &[u8]) -> Result<()> {
    let mut payload = vec![0_u8; MAX_HANDSHAKE_RECORD];
    state
        .read_message(message, &mut payload)
        .context("failed to read Noise handshake message")?;
    Ok(())
}

async fn write_preface<W>(writer: &mut W) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    writer.write_all(MAGIC).await?;
    writer.write_u16(PROTOCOL_VERSION).await?;
    writer.flush().await?;
    Ok(())
}

async fn read_preface<R>(reader: &mut R) -> Result<()>
where
    R: AsyncRead + Unpin,
{
    let mut magic = [0_u8; 4];
    reader.read_exact(&mut magic).await?;
    ensure!(&magic == MAGIC, "invalid tunnel protocol magic");
    let version = reader.read_u16().await?;
    ensure!(
        version == PROTOCOL_VERSION,
        "unsupported tunnel protocol version {version}; expected {PROTOCOL_VERSION}"
    );
    Ok(())
}

async fn write_record<W>(writer: &mut W, bytes: &[u8]) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    ensure!(bytes.len() <= u16::MAX as usize, "record is too large");
    writer.write_u16(bytes.len() as u16).await?;
    writer.write_all(bytes).await?;
    writer.flush().await?;
    Ok(())
}

async fn read_record<R>(reader: &mut R, maximum: usize) -> Result<Vec<u8>>
where
    R: AsyncRead + Unpin,
{
    let length = reader.read_u16().await? as usize;
    ensure!(length > 0, "empty encrypted record");
    ensure!(length <= maximum, "encrypted record exceeds size limit");
    let mut bytes = vec![0_u8; length];
    reader.read_exact(&mut bytes).await?;
    Ok(bytes)
}

async fn pump_outgoing<R, W>(
    mut reader: ReadHalf<R>,
    mut writer: WriteHalf<W>,
    state: Arc<Mutex<TransportState>>,
) -> Result<()>
where
    R: AsyncRead + AsyncWrite + Unpin,
    W: AsyncRead + AsyncWrite + Unpin,
{
    let mut plaintext = vec![0_u8; MAX_PLAINTEXT_RECORD];
    let mut ciphertext = vec![0_u8; MAX_PLAINTEXT_RECORD + AUTH_TAG_LEN];
    loop {
        let read = reader.read(&mut plaintext).await?;
        if read == 0 {
            writer.shutdown().await?;
            return Ok(());
        }
        let encrypted = {
            let mut state = state
                .lock()
                .map_err(|_| anyhow!("Noise transport lock poisoned"))?;
            state
                .write_message(&plaintext[..read], &mut ciphertext)
                .context("Noise encryption failed")?
        };
        write_record(&mut writer, &ciphertext[..encrypted]).await?;
    }
}

async fn pump_incoming<R, W>(
    mut reader: ReadHalf<R>,
    mut writer: WriteHalf<W>,
    state: Arc<Mutex<TransportState>>,
) -> Result<()>
where
    R: AsyncRead + AsyncWrite + Unpin,
    W: AsyncRead + AsyncWrite + Unpin,
{
    loop {
        let ciphertext = match read_transport_record(&mut reader).await? {
            Some(record) => record,
            None => {
                writer.shutdown().await?;
                return Ok(());
            }
        };
        let mut plaintext = vec![0_u8; ciphertext.len()];
        let decrypted = {
            let mut state = state
                .lock()
                .map_err(|_| anyhow!("Noise transport lock poisoned"))?;
            state
                .read_message(&ciphertext, &mut plaintext)
                .context("Noise decryption failed")?
        };
        writer.write_all(&plaintext[..decrypted]).await?;
        writer.flush().await?;
    }
}

async fn read_transport_record<R>(reader: &mut R) -> Result<Option<Vec<u8>>>
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
    ensure!(
        (AUTH_TAG_LEN..=MAX_PLAINTEXT_RECORD + AUTH_TAG_LEN).contains(&length),
        "invalid encrypted transport record length {length}"
    );
    let mut ciphertext = vec![0_u8; length];
    reader.read_exact(&mut ciphertext).await?;
    Ok(Some(ciphertext))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn wrong_psk_is_rejected() {
        let (mut client, mut server) = tokio::io::duplex(4096);
        let client_task =
            tokio::spawn(async move { client_handshake(&mut client, &[1; 32]).await });
        let server_result = server_handshake(&mut server, &[2; 32]).await;
        assert!(server_result.is_err());
        drop(server);
        assert!(client_task.await.unwrap().is_err());
    }

    #[tokio::test]
    async fn encrypted_transport_round_trip_and_tamper_detection() {
        let psk = [7_u8; 32];
        let (mut client_io, mut server_io) = tokio::io::duplex(4096);
        let (client_state, server_state) = tokio::join!(
            client_handshake(&mut client_io, &psk),
            server_handshake(&mut server_io, &psk)
        );
        let mut client_state = client_state.unwrap();
        let mut server_state = server_state.unwrap();

        let mut encrypted = [0_u8; 128];
        let size = client_state
            .write_message(b"secret", &mut encrypted)
            .unwrap();
        encrypted[size - 1] ^= 1;
        let mut plaintext = [0_u8; 128];
        assert!(
            server_state
                .read_message(&encrypted[..size], &mut plaintext)
                .is_err()
        );
    }

    #[tokio::test]
    async fn rejects_an_unsupported_tunnel_version() {
        let (mut peer, mut server) = tokio::io::duplex(64);
        let peer_task = tokio::spawn(async move {
            peer.write_all(MAGIC).await.unwrap();
            peer.write_u16(PROTOCOL_VERSION + 1).await.unwrap();
        });
        let error = server_handshake(&mut server, &[1; 32]).await.unwrap_err();
        assert!(
            error
                .to_string()
                .contains("unsupported tunnel protocol version")
        );
        peer_task.await.unwrap();
    }
}
