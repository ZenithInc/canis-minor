use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    net::SocketAddrV4,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use anyhow::{Context, Result, anyhow};
use canis_protocol::{
    mux::MuxHandle,
    wire::{StreamKind, StreamRequest, read_datagram, read_response, write_datagram},
};
use socket2::SockRef;
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
    sync::{Mutex, Semaphore, mpsc},
    time::timeout,
};
use tokio_util::{compat::FuturesAsyncReadCompatExt, sync::CancellationToken};
use tracing::{debug, warn};

use crate::transparent::{TransparentUdpSocket, original_tcp_target, udp_response_socket};

#[derive(Clone, Copy, Debug, Eq)]
struct UdpKey {
    client: SocketAddrV4,
    target: SocketAddrV4,
}

impl PartialEq for UdpKey {
    fn eq(&self, other: &Self) -> bool {
        self.client == other.client && self.target == other.target
    }
}

impl Hash for UdpKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.client.hash(state);
        self.target.hash(state);
    }
}

#[derive(Debug)]
struct UdpEntry {
    id: u64,
    datagrams: mpsc::Sender<Vec<u8>>,
}

pub async fn run_tcp(
    listener: TcpListener,
    mux: MuxHandle,
    limit: Arc<Semaphore>,
    bypass_mark: u32,
    cancellation: CancellationToken,
) -> Result<()> {
    loop {
        tokio::select! {
            _ = cancellation.cancelled() => return Ok(()),
            accepted = listener.accept() => {
                let (socket, client) = accepted?;
                let target = original_tcp_target(&socket)?;
                SockRef::from(&socket).set_mark(bypass_mark)?;
                socket.set_nodelay(true)?;
                let Ok(permit) = Arc::clone(&limit).try_acquire_owned() else {
                    warn!(%client, %target, "TCP stream limit reached; dropping connection");
                    continue;
                };
                let stream_mux = mux.clone();
                let stream_cancellation = cancellation.clone();
                tokio::spawn(async move {
                    let _permit = permit;
                    if let Err(error) = proxy_tcp(socket, target, stream_mux, stream_cancellation).await {
                        debug!(%client, %target, error = %error, "TCP proxy stream ended");
                    }
                });
            }
        }
    }
}

async fn proxy_tcp(
    mut local: TcpStream,
    target: SocketAddrV4,
    mux: MuxHandle,
    cancellation: CancellationToken,
) -> Result<()> {
    let stream = tokio::select! {
        _ = cancellation.cancelled() => return Ok(()),
        stream = mux.open_stream() => stream?,
    };
    let mut tunnel = stream.compat();
    StreamRequest {
        kind: StreamKind::Tcp,
        target,
    }
    .write_to(&mut tunnel)
    .await?;
    read_response(&mut tunnel)
        .await
        .with_context(|| format!("server could not connect to {target}"))?;

    let copied = tokio::select! {
        _ = cancellation.cancelled() => return Ok(()),
        copied = tokio::io::copy_bidirectional(&mut local, &mut tunnel) => copied?,
    };
    debug!(%target, to_server = copied.0, to_client = copied.1, "TCP proxy complete");
    Ok(())
}

pub async fn run_udp(
    socket: Arc<TransparentUdpSocket>,
    mux: MuxHandle,
    limit: Arc<Semaphore>,
    bypass_mark: u32,
    idle_timeout: Duration,
    cancellation: CancellationToken,
) -> Result<()> {
    let sessions = Arc::new(Mutex::new(HashMap::<UdpKey, UdpEntry>::new()));
    let next_id = AtomicU64::new(1);
    let mut buffer = vec![0_u8; 65_535];

    loop {
        let (size, client, target) = tokio::select! {
            _ = cancellation.cancelled() => return Ok(()),
            packet = socket.recv_from_original(&mut buffer) => packet?,
        };
        let key = UdpKey { client, target };
        let mut payload = Some(buffer[..size].to_vec());

        let existing = {
            let sessions = sessions.lock().await;
            sessions.get(&key).map(|entry| entry.datagrams.clone())
        };
        if let Some(sender) = existing {
            match sender.try_send(payload.take().expect("UDP payload is present")) {
                Ok(()) => continue,
                Err(mpsc::error::TrySendError::Full(_)) => {
                    warn!(%client, %target, "UDP session queue is full; dropping datagram");
                    continue;
                }
                Err(mpsc::error::TrySendError::Closed(returned)) => {
                    payload = Some(returned);
                    sessions.lock().await.remove(&key);
                }
            }
        }

        let Ok(permit) = Arc::clone(&limit).try_acquire_owned() else {
            warn!(%client, %target, "UDP session limit reached; dropping datagram");
            continue;
        };
        let (sender, receiver) = mpsc::channel(64);
        sender
            .try_send(payload.expect("UDP payload was not consumed"))
            .map_err(|_| anyhow!("new UDP session queue unexpectedly full"))?;
        let id = next_id.fetch_add(1, Ordering::Relaxed);
        sessions.lock().await.insert(
            key,
            UdpEntry {
                id,
                datagrams: sender,
            },
        );

        let session_mux = mux.clone();
        let session_map = Arc::clone(&sessions);
        let session_cancellation = cancellation.clone();
        tokio::spawn(async move {
            let _permit = permit;
            if let Err(error) = proxy_udp(
                key,
                receiver,
                session_mux,
                bypass_mark,
                idle_timeout,
                session_cancellation,
            )
            .await
            {
                debug!(client = %key.client, target = %key.target, error = %error, "UDP proxy session ended");
            }
            let mut sessions = session_map.lock().await;
            if sessions.get(&key).is_some_and(|entry| entry.id == id) {
                sessions.remove(&key);
            }
        });
    }
}

async fn proxy_udp(
    key: UdpKey,
    mut local_datagrams: mpsc::Receiver<Vec<u8>>,
    mux: MuxHandle,
    bypass_mark: u32,
    idle_timeout: Duration,
    cancellation: CancellationToken,
) -> Result<()> {
    let stream = tokio::select! {
        _ = cancellation.cancelled() => return Ok(()),
        stream = mux.open_stream() => stream?,
    };
    let mut tunnel = stream.compat();
    StreamRequest {
        kind: StreamKind::Udp,
        target: key.target,
    }
    .write_to(&mut tunnel)
    .await?;
    read_response(&mut tunnel)
        .await
        .with_context(|| format!("server could not create UDP session to {}", key.target))?;
    let response = udp_response_socket(key.target, bypass_mark)?;

    loop {
        let activity = timeout(idle_timeout, async {
            tokio::select! {
                _ = cancellation.cancelled() => Ok::<bool, anyhow::Error>(false),
                datagram = local_datagrams.recv() => {
                    let Some(datagram) = datagram else { return Ok(false); };
                    write_datagram(&mut tunnel, &datagram).await?;
                    Ok(true)
                }
                datagram = read_datagram(&mut tunnel) => {
                    let Some(datagram) = datagram? else { return Ok(false); };
                    response.send_to(&datagram, key.client).await?;
                    Ok(true)
                }
            }
        })
        .await;

        match activity {
            Ok(Ok(true)) => {}
            Ok(Ok(false)) | Err(_) => {
                tunnel.shutdown().await?;
                return Ok(());
            }
            Ok(Err(error)) => return Err(error),
        }
    }
}
