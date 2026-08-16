use std::{io, time::Duration};

use anyhow::{Result, anyhow};
use futures::future::poll_fn;
use tokio::sync::{mpsc, oneshot};
use yamux::{Config, Connection, ConnectionError, Mode, Stream};

#[derive(Clone, Debug)]
pub struct MuxHandle {
    requests: mpsc::Sender<OpenRequest>,
}

#[derive(Debug)]
pub struct OpenRequest {
    response: oneshot::Sender<Result<Stream, String>>,
}

impl MuxHandle {
    pub async fn open_stream(&self) -> Result<Stream> {
        let (response, result) = oneshot::channel();
        self.requests
            .send(OpenRequest { response })
            .await
            .map_err(|_| anyhow!("tunnel is disconnected"))?;
        result
            .await
            .map_err(|_| anyhow!("tunnel stopped while opening a stream"))?
            .map_err(|error| anyhow!(error))
    }
}

pub fn channel(capacity: usize) -> (MuxHandle, mpsc::Receiver<OpenRequest>) {
    let (requests, receiver) = mpsc::channel(capacity);
    (MuxHandle { requests }, receiver)
}

pub fn config(max_streams: usize) -> Config {
    let mut config = Config::default();
    config.set_max_num_streams(max_streams);
    config.set_max_connection_receive_window(Some(max_streams * 256 * 1024));
    config
}

pub async fn drive_client<T>(
    io: T,
    mut requests: mpsc::Receiver<OpenRequest>,
    max_streams: usize,
    heartbeat: Duration,
) -> Result<()>
where
    T: futures::AsyncRead + futures::AsyncWrite + Unpin,
{
    let mut connection = Connection::new(io, config(max_streams), Mode::Client);
    let mut ticker = tokio::time::interval(heartbeat);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                // Re-entering poll_next_inbound below drives Yamux's built-in ping.
            }
            request = requests.recv() => {
                let Some(request) = request else {
                    poll_fn(|cx| connection.poll_close(cx)).await?;
                    return Ok(());
                };
                match poll_fn(|cx| connection.poll_new_outbound(cx)).await {
                    Ok(stream) => {
                        let _ = request.response.send(Ok(stream));
                    }
                    Err(ConnectionError::TooManyStreams) => {
                        let _ = request.response.send(Err("maximum number of streams reached".into()));
                    }
                    Err(error) => {
                        let message = error.to_string();
                        let _ = request.response.send(Err(message.clone()));
                        return Err(anyhow!(message));
                    }
                }
            }
            inbound = poll_fn(|cx| connection.poll_next_inbound(cx)) => {
                match inbound {
                    Some(Ok(stream)) => {
                        drop(stream);
                        return Err(anyhow!("server opened an unexpected Yamux stream"));
                    }
                    Some(Err(error)) => return Err(error.into()),
                    None => return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Yamux connection closed").into()),
                }
            }
        }
    }
}
