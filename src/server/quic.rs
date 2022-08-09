use color_eyre::eyre::Result;
use futures_util::StreamExt;
use quinn::{Endpoint, NewConnection};

use super::receive_packets;
use crate::{
  ext::{EitherExt, ResultExt},
  server_addr,
};

pub async fn quic(server_config: quinn::ServerConfig) -> Result<()> {
  let (_, incoming) = Endpoint::server(server_config, server_addr())?;
  tokio::spawn(async move {
    handle_incoming(incoming).await.unwrap();
  });
  Ok(())
}
pub async fn handle_incoming(mut incoming: quinn::Incoming) -> Result<()> {
  while let Some(conn) = incoming.next().await {
    let mut nconn: NewConnection = match conn.await {
      Ok(v) => v,
      Err(e) => {
        error!("{}", e);
        continue;
      }
    };
    let conn = nconn.connection;

    tokio::spawn(async move {
      while let Some(Ok(recv)) = nconn.uni_streams.next().await {
        let conn_clone = conn.clone();
        tokio::spawn(async move {
          if let Some(bytes) = recv.read_to_end(1024).await.log() {
            let conn_id = conn_clone.stable_id().tl();
            receive_packets(bytes, conn_clone.tl(), conn_id).await.log();
          }
        });
      }
    });
  }
  Ok(())
}
