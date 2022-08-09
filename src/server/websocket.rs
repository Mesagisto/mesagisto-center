use std::sync::Arc;

use color_eyre::eyre::Result;
use futures_util::{SinkExt, StreamExt};
use tokio::{
  net::TcpListener,
  sync::mpsc
};
use tokio_tungstenite::tungstenite as ws;

use crate::{
  ext::{EitherExt, ResultExt, EyreExt},
  server::receive_packets,
  ws_server_addr,
};

pub async fn ws() -> Result<()> {
  // Create the event loop and TCP listener we'll accept connections on.
  let listener = TcpListener::bind(&ws_server_addr()).await?;
  tokio::spawn(async move {
    while let Ok((stream, _)) = listener.accept().await {
      tokio::spawn(accept_connection(stream));
    }
  });
  Ok(())
}

pub async fn accept_connection(stream: tokio::net::TcpStream) -> Result<()> {
  let addr = stream.peer_addr()?;
  // handshake happens here
  let ws_stream = tokio_tungstenite::accept_async(stream).await?;

  info!("New WebSocket connection: {}", addr);

  let conn_id = Arc::new(addr);

  let (tx, mut rx) = mpsc::channel(64);
  let (write, mut read) = ws_stream.split();
  tokio::spawn(async move {
    let mut write = write;
    while let Some(ws_message) = rx.recv().await {
      match write.send(ws_message).await {
        Err(ws::Error::ConnectionClosed) | Err(ws::Error::AlreadyClosed) => {
          break;
        }
        Err(ws::Error::Protocol(ws::error::ProtocolError::ResetWithoutClosingHandshake)) => {
          break;
        }
        Err(e) => error!("{:?}", e),
        _ => {}
      };
    }
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
        Err(ws::Error::Protocol(ws::error::ProtocolError::ResetWithoutClosingHandshake)) => {
          break;
        }
        Err(e) => error!("{:?}", e.to_eyre()),
        Ok(ws::Message::Pong(_)) => {
          debug!("pong from {}", conn_id);
        }
        Ok(ws::Message::Binary(data)) => {
          tokio::spawn(async move {
            receive_packets(data, tx.tr(), conn_id.tr()).await.log();
          });
        }
        Ok(msg) => warn!("unexpected message {}", msg),
      }
    }
  });
  Ok(())
}

