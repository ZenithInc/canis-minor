mod network;
mod proxy;
mod transparent;

use std::{
    os::fd::AsRawFd,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow, ensure};
use canis_protocol::{
    config::{ClientConfig, load_psk},
    crypto::{client_handshake, spawn_encrypted_transport},
    mux,
};
use clap::{Parser, Subcommand};
use network::{NetworkRules, flush_resolved_cache};
use socket2::{SockRef, TcpKeepalive};
use tokio::{
    net::{TcpSocket, TcpStream},
    sync::Semaphore,
    time::timeout,
};
use tokio_util::{compat::TokioAsyncReadCompatExt, sync::CancellationToken};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use crate::{
    proxy::{run_tcp, run_udp},
    transparent::{TransparentUdpSocket, set_mark, tcp_listener},
};

#[derive(Debug, Parser)]
#[command(version, about = "Canis Minor transparent-tunnel client")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Run {
        #[arg(long, default_value = "/etc/canis-minor/client.toml")]
        config: PathBuf,
    },
    Cleanup {
        #[arg(long, default_value = "/etc/canis-minor/client.toml")]
        config: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
    let command = Cli::parse().command;
    ensure_root()?;
    match command {
        Command::Run { config } => {
            let config = Arc::new(ClientConfig::load(config)?);
            let psk = Arc::new(load_psk(&config.psk_file)?);
            run(config, psk).await
        }
        Command::Cleanup { config } => {
            let config = ClientConfig::load(config)?;
            NetworkRules::new(config).cleanup().await
        }
    }
}

async fn run(config: Arc<ClientConfig>, psk: Arc<[u8; 32]>) -> Result<()> {
    let rules = NetworkRules::new((*config).clone());
    rules
        .cleanup()
        .await
        .context("startup network cleanup failed")?;

    let shutdown = CancellationToken::new();
    let signal_shutdown = shutdown.clone();
    tokio::spawn(async move {
        wait_for_shutdown().await;
        signal_shutdown.cancel();
    });

    let mut retry_delay = Duration::from_secs(config.reconnect_min_secs);
    let maximum_delay = Duration::from_secs(config.reconnect_max_secs);
    while !shutdown.is_cancelled() {
        let started = Instant::now();
        let result = run_session(
            Arc::clone(&config),
            Arc::clone(&psk),
            rules.clone(),
            shutdown.clone(),
        )
        .await;
        rules.cleanup().await?;

        if shutdown.is_cancelled() {
            break;
        }
        if let Err(error) = result {
            warn!(error = ?error, retry_in = ?retry_delay, "tunnel disconnected");
        }
        if started.elapsed() >= Duration::from_secs(30) {
            retry_delay = Duration::from_secs(config.reconnect_min_secs);
        }
        tokio::select! {
            _ = shutdown.cancelled() => break,
            _ = tokio::time::sleep(retry_delay) => {}
        }
        retry_delay = retry_delay.saturating_mul(2).min(maximum_delay);
    }

    rules.cleanup().await?;
    info!("client stopped and transparent-proxy rules were removed");
    Ok(())
}

async fn run_session(
    config: Arc<ClientConfig>,
    psk: Arc<[u8; 32]>,
    rules: NetworkRules,
    shutdown: CancellationToken,
) -> Result<()> {
    #[derive(Clone, Copy)]
    enum FinishedTask {
        Shutdown,
        Tcp,
        Udp,
        Mux,
        Encryption,
    }

    let mut socket = connect_marked(&config).await?;
    let transport = timeout(
        config.handshake_timeout(),
        client_handshake(&mut socket, &psk),
    )
    .await
    .context("Noise handshake timed out")??;

    let tcp = tcp_listener(config.proxy_port, config.bypass_mark)
        .context("transparent TCP listener setup failed")?;
    let udp = Arc::new(
        TransparentUdpSocket::bind(config.proxy_port, config.bypass_mark)
            .context("transparent UDP listener setup failed")?,
    );

    let (secure_io, mut encryption_pump) = spawn_encrypted_transport(socket, transport);
    let (mux_handle, requests) = mux::channel(config.max_streams);
    let mut mux_task = tokio::spawn(mux::drive_client(
        secure_io.compat(),
        requests,
        config.max_streams,
        Duration::from_secs(config.heartbeat_secs),
    ));

    let proxy_cancellation = CancellationToken::new();
    let limit = Arc::new(Semaphore::new(config.max_streams));
    let mut tcp_task = tokio::spawn(run_tcp(
        tcp,
        mux_handle.clone(),
        Arc::clone(&limit),
        config.bypass_mark,
        proxy_cancellation.clone(),
    ));
    let mut udp_task = tokio::spawn(run_udp(
        udp,
        mux_handle,
        limit,
        config.bypass_mark,
        config.udp_idle_timeout(),
        proxy_cancellation.clone(),
    ));

    if let Err(error) = rules.install().await {
        proxy_cancellation.cancel();
        tcp_task.abort();
        udp_task.abort();
        mux_task.abort();
        encryption_pump.abort();
        let _ = tokio::join!(tcp_task, udp_task, mux_task, encryption_pump);
        return Err(error);
    }
    flush_resolved_cache().await;
    info!(server = %config.server_addr, "authenticated tunnel active; transparent proxy enabled");

    let (result, finished) = tokio::select! {
        _ = shutdown.cancelled() => (Ok(()), FinishedTask::Shutdown),
        result = &mut tcp_task => (match result {
            Ok(Ok(())) => Err(anyhow!("transparent TCP listener stopped")),
            Ok(Err(error)) => Err(error.context("transparent TCP listener failed")),
            Err(error) => Err(anyhow!(error).context("transparent TCP listener task panicked")),
        }, FinishedTask::Tcp),
        result = &mut udp_task => (match result {
            Ok(Ok(())) => Err(anyhow!("transparent UDP listener stopped")),
            Ok(Err(error)) => Err(error.context("transparent UDP listener failed")),
            Err(error) => Err(anyhow!(error).context("transparent UDP listener task panicked")),
        }, FinishedTask::Udp),
        result = &mut mux_task => match result {
            Ok(Ok(())) => (Err(anyhow!("Yamux manager stopped")), FinishedTask::Mux),
            Ok(Err(error)) => (Err(error.context("Yamux manager stopped")), FinishedTask::Mux),
            Err(error) => (Err(anyhow!(error).context("Yamux manager task panicked")), FinishedTask::Mux),
        },
        result = &mut encryption_pump => match result {
            Ok(Ok(())) => (Err(anyhow!("encrypted transport stopped")), FinishedTask::Encryption),
            Ok(Err(error)) => (Err(error.context("encrypted transport stopped")), FinishedTask::Encryption),
            Err(error) => (Err(anyhow!(error).context("encryption pump task panicked")), FinishedTask::Encryption),
        },
    };

    proxy_cancellation.cancel();
    tcp_task.abort();
    udp_task.abort();
    mux_task.abort();
    encryption_pump.abort();
    match finished {
        FinishedTask::Tcp => {
            let _ = udp_task.await;
        }
        FinishedTask::Udp => {
            let _ = tcp_task.await;
        }
        FinishedTask::Shutdown | FinishedTask::Mux | FinishedTask::Encryption => {
            let _ = tokio::join!(tcp_task, udp_task);
        }
    }
    rules.cleanup().await?;
    result
}

async fn connect_marked(config: &ClientConfig) -> Result<TcpStream> {
    let socket = TcpSocket::new_v4()?;
    set_mark(socket.as_raw_fd(), config.bypass_mark)
        .context("failed to set bypass mark on tunnel socket")?;
    let stream = timeout(
        config.handshake_timeout(),
        socket.connect(config.server_addr.into()),
    )
    .await
    .context("tunnel TCP connection timed out")?
    .with_context(|| format!("failed to connect to tunnel server {}", config.server_addr))?;
    stream.set_nodelay(true)?;
    let keepalive = TcpKeepalive::new()
        .with_time(Duration::from_secs(config.heartbeat_secs))
        .with_interval(Duration::from_secs(config.heartbeat_secs))
        .with_retries(3);
    SockRef::from(&stream)
        .set_tcp_keepalive(&keepalive)
        .context("failed to configure tunnel keepalive")?;
    Ok(stream)
}

async fn wait_for_shutdown() {
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("failed to register SIGTERM handler");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = terminate.recv() => {}
    }
}

fn ensure_root() -> Result<()> {
    let uid = unsafe {
        // SAFETY: geteuid has no preconditions.
        libc::geteuid()
    };
    ensure!(uid == 0, "canis-client must run as root");
    Ok(())
}
