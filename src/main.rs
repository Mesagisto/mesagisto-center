#![feature(let_chains)]

mod config;
mod data;
mod ext;
mod log;
mod room;
pub mod server;

mod tls;

use std::net::SocketAddr;
use color_eyre::eyre::Result;
use config::Config;


use crate::{config::CONFIG, ext::ResultExt};

#[macro_use]
extern crate educe;
#[macro_use]
extern crate automatic_config;
// #[macro_use]
// extern crate singleton;
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
  Config::reload().await?;
  if !CONFIG.enable {
    warn!("MesagistoCenter is not enabled, about to exit the program.");
    warn!("To enable, please modify the configuration file.");
    return Ok(());
  }
  let certs = tls::read_certs_from_file().await?;
  let certs_clone = certs.clone();
  tokio::spawn(async move {
    server::quic(&certs_clone).await.eyre_log();
  });
  let certs_clone = certs;
  if CONFIG.tls.wss {
    tokio::spawn(async move{
      server::wss(&certs_clone).await.eyre_log();
    });
  } else {
    tokio::spawn(async move{
      server::ws().await.eyre_log();
    });
  }

  info!("Start successfully");
  tokio::signal::ctrl_c().await?;
  Ok(())
}

fn quic_server_addr() -> SocketAddr {
  CONFIG.server.quic.as_str().parse::<SocketAddr>().unwrap()
}
fn ws_server_addr() -> SocketAddr {
  CONFIG.server.ws.as_str().parse::<SocketAddr>().unwrap()
}
