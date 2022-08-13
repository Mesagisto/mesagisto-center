pub mod quic;
pub mod websocket;

use std::{net::SocketAddr, sync::Arc, time::Duration};

use color_eyre::eyre::Result;
use either::Either;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::tungstenite;

use crate::{
  data::{Ctl, Packet},
  room::ROOMS, ext::ResultExt,
};

pub type QuicOrWsConn = Either<quinn::Connection, Sender<tungstenite::Message>>;
pub type QuicOrWsConnId = Either<usize, Arc<SocketAddr>>;

pub use quic::quic;
pub use websocket::wss;
pub use websocket::ws;

pub async fn receive_packets(
  data: Vec<u8>,
  conn: QuicOrWsConn,
  conn_id: QuicOrWsConnId,
) -> Result<()> {
  let pkt: Packet = ciborium::de::from_reader(&*data)?;
  #[cfg(debug_assertions)]
  info!("uni recv: {:?}", pkt.room_id.clone());
  if let Some(ctl) = &pkt.ctl {
    match ctl {
      Ctl::Sub => {
        ROOMS.join(pkt.room_id.clone(), conn, conn_id);
      }
      Ctl::Unsub => {
        ROOMS.leave(pkt.room_id.clone(), conn_id);
      }
    }
  } else {
    tokio::time::timeout(Duration::from_secs(7), ROOMS.send(pkt.room_id, conn_id, data)).await.eyre_log();
  }
  Ok(())
}
