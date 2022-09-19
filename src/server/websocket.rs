use std::{net::SocketAddr, sync::Arc};

use color_eyre::eyre::Result;
use futures_util::{SinkExt, StreamExt};
use rustls::{Certificate, PrivateKey};
use tokio::{
  io::{AsyncRead, AsyncWrite},
  net::TcpListener,
  sync::mpsc,
};
use tokio_tungstenite::tungstenite as ws;

use crate::{
  ext::{EitherExt, EyreExt, ResultExt},
  server::receive_packets,
  ws_server_addr,
};

pub async fn wss(certs: &(Vec<Certificate>, PrivateKey)) -> Result<()> {
  let listener = TcpListener::bind(&ws_server_addr()).await?;
  let config = rustls::ServerConfig::builder()
    .with_safe_defaults()
    .with_no_client_auth()
    .with_single_cert(certs.0.to_owned(), certs.1.to_owned())?;
  let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
  while let Some((stream, _)) = listener.accept().await.log() {
    let acceptor = acceptor.clone();
    if let Some(peer_address) = stream.peer_addr().log()
    && let Some(stream) = acceptor.accept(stream).await.log() {
      accept_connection(stream, peer_address).await.eyre_log();
    };
  }
  info!("wss listening stopped");
  Ok(())
}
pub async fn ws() -> Result<()> {
  let listener = TcpListener::bind(&ws_server_addr()).await?;
  while let Some((stream, _)) = listener.accept().await.log() {
    if let Some(peer_address) = stream.peer_addr().log() {
      accept_connection(stream, peer_address).await.eyre_log();
    };
  }
  info!("ws listening stopped");
  Ok(())
}

pub async fn accept_connection<S>(stream: S, peer_address: SocketAddr) -> Result<()>
where
  S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
  // handshake happens here
  let ws_stream = tokio_tungstenite::accept_async(stream).await?;

  info!("New WebSocket connection: {}", peer_address);

  let conn_id = Arc::new(peer_address);

  let (tx, mut rx) = mpsc::channel(64);
  let (write, mut read) = ws_stream.split();

  let conn_id_clone = conn_id.clone();
  tokio::spawn(async move {
    let mut write = write;
    while let Some(ws_message) = rx.recv().await {
      match write.send(ws_message).await {
        Err(ws::Error::ConnectionClosed) | Err(ws::Error::AlreadyClosed) => {
          break;
        }
        Err(ws::Error::Protocol(ws::error::ProtocolError::ResetWithoutClosingHandshake))
        | Err(ws::Error::Io(_)) => {
          break;
        }
        Err(e) => error!("{}", e.to_eyre()),
        Ok(_) => {}
      };
    }
    info!("ws disconnected {}", conn_id_clone);
    rx.close();
  });
  tokio::spawn(async move {
    let tx = tx;
    while let Some(next) = read.next().await {
      let tx = tx.clone();
      let conn_id = conn_id.clone();
      match next {
        Err(ws::Error::ConnectionClosed) | Err(ws::Error::AlreadyClosed) => {
          break;
        }
        Err(ws::Error::Protocol(ws::error::ProtocolError::ResetWithoutClosingHandshake))
        | Err(ws::Error::Io(_)) => {
          break;
        }
        Err(e) => error!("{:?}", e.to_eyre()),
        Ok(ws::Message::Pong(_)) => {}
        Ok(ws::Message::Ping(ping)) => {
          tx.send(ws::Message::Pong(ping)).await.log();
        }
        Ok(ws::Message::Binary(data)) => {
          tokio::spawn(async move {
            receive_packets(data, tx.tr(), conn_id.tr()).await.log();
          });
        }
        Ok(msg) => warn!("unexpected message {}", msg),
      }
    }
    info!("ws disconnected {}", conn_id)
  });
  Ok(())
}
