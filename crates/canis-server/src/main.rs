use std::{
    net::{Ipv4Addr, SocketAddrV4},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result, anyhow};
use canis_protocol::{
    config::{ServerConfig, load_psk},
    crypto::{server_handshake, spawn_encrypted_transport},
    mux,
    wire::{StreamKind, StreamRequest, read_datagram, write_datagram, write_ok, write_rejected},
};
use clap::Parser;
use futures::future::poll_fn;
use socket2::{SockRef, TcpKeepalive};
use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt},
    net::{TcpListener, TcpStream, UdpSocket},
    sync::{Mutex, Semaphore},
    time::timeout,
};
use tokio_util::{
    compat::{FuturesAsyncReadCompatExt, TokioAsyncReadCompatExt},
    sync::CancellationToken,
};
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;
use yamux::{Connection, Mode};

#[derive(Debug, Parser)]
#[command(version, about = "Canis Minor encrypted transparent-tunnel server")]
struct Cli {
    #[arg(long, default_value = ".canis/server.toml")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    let config = Arc::new(ServerConfig::load(&cli.config)?);
    let psk = Arc::new(load_psk(&config.psk_file)?);
    run(config, psk).await
}

async fn run(config: Arc<ServerConfig>, psk: Arc<[u8; 32]>) -> Result<()> {
    let listener = TcpListener::bind(config.listen)
        .await
        .with_context(|| format!("failed to listen on {}", config.listen))?;
    let active_session = Arc::new(Mutex::new(None::<CancellationToken>));
    let handshakes = Arc::new(Semaphore::new(64));
    info!(listen = %config.listen, "server ready");

    loop {
        tokio::select! {
            signal = tokio::signal::ctrl_c() => {
                signal.context("failed to listen for shutdown signal")?;
                info!("shutdown requested");
                if let Some(session) = active_session.lock().await.take() {
                    session.cancel();
                }
                return Ok(());
            }
            accepted = listener.accept() => {
                let (socket, peer) = accepted.context("failed to accept tunnel connection")?;
                let Ok(permit) = Arc::clone(&handshakes).try_acquire_owned() else {
                    warn!(%peer, "too many pending tunnel handshakes");
                    continue;
                };
                configure_keepalive(&socket, config.heartbeat_secs);
                let connection_config = Arc::clone(&config);
                let connection_psk = Arc::clone(&psk);
                let sessions = Arc::clone(&active_session);
                tokio::spawn(async move {
                    let _permit = permit;
                    if let Err(error) = serve_connection(socket, peer, connection_config, connection_psk, sessions).await {
                        warn!(%peer, error = %error, "tunnel session ended");
                    }
                });
            }
        }
    }
}

async fn serve_connection(
    mut socket: TcpStream,
    peer: std::net::SocketAddr,
    config: Arc<ServerConfig>,
    psk: Arc<[u8; 32]>,
    active_session: Arc<Mutex<Option<CancellationToken>>>,
) -> Result<()> {
    let transport = timeout(
        config.handshake_timeout(),
        server_handshake(&mut socket, &psk),
    )
    .await
    .context("Noise handshake timed out")??;

    let cancellation = CancellationToken::new();
    if let Some(previous) = active_session.lock().await.replace(cancellation.clone()) {
        previous.cancel();
        info!(%peer, "authenticated replacement session superseded the old session");
    }

    info!(%peer, "authenticated tunnel established");
    let (secure_io, mut encryption_pump) = spawn_encrypted_transport(socket, transport);
    let server = serve_mux(
        secure_io.compat(),
        Arc::clone(&config),
        cancellation.clone(),
    );
    tokio::pin!(server);

    let result = tokio::select! {
        result = &mut server => result,
        result = &mut encryption_pump => match result {
            Ok(Ok(())) => Err(anyhow!("encrypted transport closed")),
            Ok(Err(error)) => Err(error.context("encrypted transport failed")),
            Err(error) => Err(anyhow!(error).context("encryption pump task panicked")),
        },
    };
    cancellation.cancel();
    encryption_pump.abort();
    result
}

async fn serve_mux<T>(
    io: T,
    config: Arc<ServerConfig>,
    cancellation: CancellationToken,
) -> Result<()>
where
    T: futures::AsyncRead + futures::AsyncWrite + Unpin,
{
    let mut connection = Connection::new(io, mux::config(config.max_streams), Mode::Server);
    let streams = Arc::new(Semaphore::new(config.max_streams));
    let mut heartbeat = tokio::time::interval(Duration::from_secs(config.heartbeat_secs));
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = cancellation.cancelled() => {
                poll_fn(|cx| connection.poll_close(cx)).await?;
                return Ok(());
            }
            _ = heartbeat.tick() => {
                // Periodically repolling drives Yamux ping frames on an idle tunnel.
            }
            incoming = poll_fn(|cx| connection.poll_next_inbound(cx)) => {
                match incoming {
                    Some(Ok(stream)) => {
                        let Ok(permit) = Arc::clone(&streams).try_acquire_owned() else {
                            warn!("stream limit reached; rejecting excess stream");
                            drop(stream);
                            continue;
                        };
                        let stream_config = Arc::clone(&config);
                        tokio::spawn(async move {
                            let _permit = permit;
                            let stream = stream.compat();
                            if let Err(error) = handle_stream(stream, &stream_config).await {
                                debug!(error = %error, "proxy stream closed with an error");
                            }
                        });
                    }
                    Some(Err(error)) => return Err(error.into()),
                    None => return Err(anyhow!("Yamux connection closed")),
                }
            }
        }
    }
}

async fn handle_stream<S>(mut stream: S, config: &ServerConfig) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut request = StreamRequest::read_from(&mut stream)
        .await
        .context("invalid stream request")?;
    request.target = rewrite_dns_target(request.target, config.dns_upstream);
    match request.kind {
        StreamKind::Tcp => handle_tcp(stream, request.target, config.connect_timeout()).await,
        StreamKind::Udp => handle_udp(stream, request.target, config.udp_idle_timeout()).await,
    }
}

fn rewrite_dns_target(target: SocketAddrV4, dns_upstream: SocketAddrV4) -> SocketAddrV4 {
    if target.port() == 53 {
        dns_upstream
    } else {
        target
    }
}

async fn handle_tcp<S>(mut stream: S, target: SocketAddrV4, connect_timeout: Duration) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut outbound = match timeout(connect_timeout, TcpStream::connect(target)).await {
        Ok(Ok(socket)) => socket,
        Ok(Err(error)) => {
            write_rejected(&mut stream, &format!("connect failed: {error}")).await?;
            stream.shutdown().await?;
            return Ok(());
        }
        Err(_) => {
            write_rejected(&mut stream, "connect timed out").await?;
            stream.shutdown().await?;
            return Ok(());
        }
    };
    outbound.set_nodelay(true)?;
    write_ok(&mut stream).await?;
    let (to_server, to_client) = tokio::io::copy_bidirectional(&mut stream, &mut outbound).await?;
    debug!(%target, to_server, to_client, "TCP stream complete");
    Ok(())
}

async fn handle_udp<S>(mut stream: S, target: SocketAddrV4, idle_timeout: Duration) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let socket = match UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)).await {
        Ok(socket) => socket,
        Err(error) => {
            write_rejected(&mut stream, &format!("UDP bind failed: {error}")).await?;
            return Ok(());
        }
    };
    if let Err(error) = socket.connect(target).await {
        write_rejected(&mut stream, &format!("UDP connect failed: {error}")).await?;
        return Ok(());
    }
    write_ok(&mut stream).await?;

    let mut response = vec![0_u8; 65_535];
    loop {
        let activity = timeout(idle_timeout, async {
            tokio::select! {
                datagram = read_datagram(&mut stream) => {
                    match datagram? {
                        Some(datagram) => {
                            socket.send(&datagram).await?;
                            Ok::<bool, anyhow::Error>(true)
                        }
                        None => Ok(false),
                    }
                }
                received = socket.recv(&mut response) => {
                    let received = received?;
                    write_datagram(&mut stream, &response[..received]).await?;
                    Ok(true)
                }
            }
        })
        .await;

        match activity {
            Ok(Ok(true)) => {}
            Ok(Ok(false)) => return Ok(()),
            Ok(Err(error)) => return Err(error),
            Err(_) => {
                debug!(%target, "UDP session expired after idle timeout");
                return Ok(());
            }
        }
    }
}

fn configure_keepalive(socket: &TcpStream, heartbeat_secs: u64) {
    let settings = TcpKeepalive::new()
        .with_time(Duration::from_secs(heartbeat_secs))
        .with_interval(Duration::from_secs(heartbeat_secs))
        .with_retries(3);
    if let Err(error) = SockRef::from(socket).set_tcp_keepalive(&settings) {
        error!(%error, "failed to configure TCP keepalive");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use canis_protocol::{
        crypto::{client_handshake, server_handshake, spawn_encrypted_transport},
        mux::MuxHandle,
        wire::{StreamKind, StreamRequest, read_datagram, read_response, write_datagram},
    };
    use futures::future::join_all;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        task::JoinHandle,
    };
    use tokio_util::compat::{FuturesAsyncReadCompatExt, TokioAsyncReadCompatExt};

    struct TestTunnel {
        mux: MuxHandle,
        cancellation: CancellationToken,
        client_mux: JoinHandle<Result<()>>,
        client_pump: JoinHandle<Result<()>>,
        server_mux: JoinHandle<Result<()>>,
        server_pump: JoinHandle<Result<()>>,
    }

    impl TestTunnel {
        fn stop(self) {
            self.cancellation.cancel();
            self.client_mux.abort();
            self.client_pump.abort();
            self.server_mux.abort();
            self.server_pump.abort();
        }
    }

    fn test_config() -> ServerConfig {
        ServerConfig {
            listen: "127.0.0.1:5260".parse().unwrap(),
            psk_file: "/unused".into(),
            dns_upstream: "1.1.1.1:53".parse().unwrap(),
            handshake_timeout_secs: 2,
            connect_timeout_secs: 2,
            udp_idle_secs: 2,
            heartbeat_secs: 1,
            max_streams: 256,
        }
    }

    async fn start_test_tunnel() -> TestTunnel {
        let psk = [42_u8; 32];
        let (mut client_io, mut server_io) = tokio::io::duplex(1024 * 1024);
        let (client_state, server_state) = tokio::join!(
            client_handshake(&mut client_io, &psk),
            server_handshake(&mut server_io, &psk)
        );
        let (client_secure, client_pump) =
            spawn_encrypted_transport(client_io, client_state.unwrap());
        let (server_secure, server_pump) =
            spawn_encrypted_transport(server_io, server_state.unwrap());

        let cancellation = CancellationToken::new();
        let server_mux = tokio::spawn(serve_mux(
            server_secure.compat(),
            Arc::new(test_config()),
            cancellation.clone(),
        ));
        let (mux, requests) = mux::channel(256);
        let client_mux = tokio::spawn(mux::drive_client(
            client_secure.compat(),
            requests,
            256,
            Duration::from_secs(1),
        ));
        TestTunnel {
            mux,
            cancellation,
            client_mux,
            client_pump,
            server_mux,
            server_pump,
        }
    }

    #[tokio::test]
    async fn reports_server_connect_failure() {
        let reserved = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let target = match reserved.local_addr().unwrap() {
            std::net::SocketAddr::V4(address) => address,
            _ => unreachable!(),
        };
        drop(reserved);

        let (mut client, server) = tokio::io::duplex(1024);
        let task = tokio::spawn(handle_tcp(server, target, Duration::from_secs(1)));
        StreamRequest {
            kind: StreamKind::Tcp,
            target,
        }
        .write_to(&mut client)
        .await
        .unwrap();
        assert!(read_response(&mut client).await.is_err());
        task.await.unwrap().unwrap();
    }

    #[test]
    fn rewrites_only_port_53() {
        let dns = "1.1.1.1:53".parse().unwrap();
        assert_eq!(
            rewrite_dns_target("203.0.113.8:53".parse().unwrap(), dns),
            dns
        );
        assert_eq!(
            rewrite_dns_target("203.0.113.8:443".parse().unwrap(), dns),
            "203.0.113.8:443".parse().unwrap()
        );
    }

    #[tokio::test]
    async fn encrypted_tcp_echo_preserves_half_close() {
        timeout(Duration::from_secs(5), async {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let target = match listener.local_addr().unwrap() {
                std::net::SocketAddr::V4(address) => address,
                _ => unreachable!(),
            };
            let echo = tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                socket.read_to_end(&mut request).await.unwrap();
                socket.write_all(&request).await.unwrap();
                socket.shutdown().await.unwrap();
            });

            let tunnel = start_test_tunnel().await;
            let mut stream = tunnel.mux.open_stream().await.unwrap().compat();
            StreamRequest {
                kind: StreamKind::Tcp,
                target,
            }
            .write_to(&mut stream)
            .await
            .unwrap();
            read_response(&mut stream).await.unwrap();
            stream.write_all(b"half-close works").await.unwrap();
            stream.shutdown().await.unwrap();
            let mut echoed = Vec::new();
            stream.read_to_end(&mut echoed).await.unwrap();
            assert_eq!(echoed, b"half-close works");
            echo.await.unwrap();
            tunnel.stop();
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn encrypted_udp_echo_preserves_datagram_boundaries() {
        timeout(Duration::from_secs(5), async {
            let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
            let target = match socket.local_addr().unwrap() {
                std::net::SocketAddr::V4(address) => address,
                _ => unreachable!(),
            };
            let echo = tokio::spawn(async move {
                let mut buffer = [0_u8; 1024];
                for _ in 0..2 {
                    let (size, peer) = socket.recv_from(&mut buffer).await.unwrap();
                    socket.send_to(&buffer[..size], peer).await.unwrap();
                }
            });

            let tunnel = start_test_tunnel().await;
            let mut stream = tunnel.mux.open_stream().await.unwrap().compat();
            StreamRequest {
                kind: StreamKind::Udp,
                target,
            }
            .write_to(&mut stream)
            .await
            .unwrap();
            read_response(&mut stream).await.unwrap();
            write_datagram(&mut stream, b"one").await.unwrap();
            write_datagram(&mut stream, b"a different length")
                .await
                .unwrap();
            assert_eq!(read_datagram(&mut stream).await.unwrap().unwrap(), b"one");
            assert_eq!(
                read_datagram(&mut stream).await.unwrap().unwrap(),
                b"a different length"
            );
            echo.await.unwrap();
            tunnel.stop();
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn supports_concurrent_encrypted_tcp_streams() {
        timeout(Duration::from_secs(10), async {
            const COUNT: usize = 32;
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let target = match listener.local_addr().unwrap() {
                std::net::SocketAddr::V4(address) => address,
                _ => unreachable!(),
            };
            let echo = tokio::spawn(async move {
                let mut tasks = Vec::new();
                for _ in 0..COUNT {
                    let (mut socket, _) = listener.accept().await.unwrap();
                    tasks.push(tokio::spawn(async move {
                        let mut bytes = [0_u8; 4];
                        socket.read_exact(&mut bytes).await.unwrap();
                        socket.write_all(&bytes).await.unwrap();
                    }));
                }
                join_all(tasks).await;
            });

            let tunnel = start_test_tunnel().await;
            let tasks = (0..COUNT).map(|index| {
                let mux = tunnel.mux.clone();
                tokio::spawn(async move {
                    let mut stream = mux.open_stream().await.unwrap().compat();
                    StreamRequest {
                        kind: StreamKind::Tcp,
                        target,
                    }
                    .write_to(&mut stream)
                    .await
                    .unwrap();
                    read_response(&mut stream).await.unwrap();
                    let request = (index as u32).to_be_bytes();
                    stream.write_all(&request).await.unwrap();
                    let mut response = [0_u8; 4];
                    stream.read_exact(&mut response).await.unwrap();
                    assert_eq!(response, request);
                })
            });
            for result in join_all(tasks).await {
                result.unwrap();
            }
            echo.await.unwrap();
            tunnel.stop();
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn tunnel_interruption_is_reported_and_a_new_tunnel_can_start() {
        timeout(Duration::from_secs(5), async {
            let first = start_test_tunnel().await;
            first.server_pump.abort();
            let client_result = first.client_mux.await.unwrap();
            assert!(client_result.is_err());
            first.cancellation.cancel();
            first.client_pump.abort();
            first.server_mux.abort();

            let second = start_test_tunnel().await;
            let stream = second.mux.open_stream().await.unwrap();
            drop(stream);
            second.stop();
        })
        .await
        .unwrap();
    }
}
