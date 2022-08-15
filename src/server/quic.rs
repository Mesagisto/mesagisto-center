use std::{sync::Arc, time::Duration};

use color_eyre::eyre::Result;
use futures_util::StreamExt;
use quinn::{Endpoint, NewConnection, TransportConfig, IdleTimeout};
use rustls::{Certificate, PrivateKey};

use super::receive_packets;
use crate::{
  ext::{EitherExt, ResultExt},
  quic_server_addr,
};

pub async fn quic(certs: &(Vec<Certificate>, PrivateKey)) -> Result<()> {
  let mut server_config =
    quinn::ServerConfig::with_single_cert(certs.0.to_owned(), certs.1.to_owned())?;
  let mut trans_config = TransportConfig::default();
  trans_config.keep_alive_interval(Some(Duration::from_secs(5)));
  trans_config.max_idle_timeout(Some(IdleTimeout::try_from(Duration::from_secs(8))?));
  server_config.transport = Arc::new(trans_config);
  let mut cert_store = rustls::RootCertStore::empty();
  for cert in &certs.0 {
    cert_store.add(cert)?
  }
  let (_, incoming) = Endpoint::server(server_config, quic_server_addr())?;
  handle_incoming(incoming).await.unwrap();
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
      info!("quic connection close")
    });
  }
  info!("quic listening stopped");
  Ok(())
}
