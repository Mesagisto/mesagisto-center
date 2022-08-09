#![feature(let_chains)]

mod data;
mod ext;
mod log;
mod room;
pub mod server;
mod tls;

use std::{net::SocketAddr, sync::Arc, time::Duration};

use color_eyre::eyre::Result;
use quinn::TransportConfig;

#[macro_use]
extern crate tracing;

#[tokio::main]
async fn main() -> Result<()> {
  run().await?;
  Ok(())
}

async fn run() -> Result<()> {
  #[cfg(debug_assertions)]
  std::env::set_var("RUST_BACKTRACE", "full");
  #[cfg(not(debug_assertions))]
  std::env::set_var("RUST_BACKTRACE", "1");

  if cfg!(feature = "color") {
    color_eyre::install()?;
  } else {
    color_eyre::config::HookBuilder::new()
      .theme(color_eyre::config::Theme::new())
      .install()?;
  }
  log::init().await?;

  let certs = tls::read_certs_from_file()?;
  let mut server_config = quinn::ServerConfig::with_single_cert(certs.0.to_owned(), certs.1)?;
  let mut trans_config = TransportConfig::default();
  trans_config.keep_alive_interval(Some(Duration::from_secs(5)));
  server_config.transport = Arc::new(trans_config);
  let mut cert_store = rustls::RootCertStore::empty();
  for cert in certs.0 {
    cert_store.add(&cert)?
  }

  server::quic(server_config).await.unwrap();

  tokio::spawn(async {
    server::ws().await.unwrap();
  });
  tokio::signal::ctrl_c().await?;
  Ok(())
}

fn server_addr() -> SocketAddr {
  "127.0.0.1:6996".parse::<SocketAddr>().unwrap()
}
fn ws_server_addr() -> SocketAddr {
  server_addr()
}
