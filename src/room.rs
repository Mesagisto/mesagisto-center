use std::{ops::Deref, sync::Arc};

use dashmap::DashMap;
use either::Either;
use futures_util::{future::join_all, stream::FuturesUnordered};
use singleton::Singleton;
use tokio::sync::mpsc::error::TrySendError;
use tokio_tungstenite::tungstenite;
use uuid::Uuid;

use crate::{
  ext::ResultExt,
  server::{QuicOrWsConn, QuicOrWsConnId},
};

#[derive(Singleton, Default)]
pub struct Rooms {
  pub inner: DashMap<Arc<Uuid>, Room>,
}
impl Rooms {
  pub async fn send(&self, room: Arc<Uuid>, conn_id: QuicOrWsConnId, pkt: Vec<u8>) {
    if let Some(room) = self.inner.get(&room) {
      room.send(conn_id, pkt).await;
    };
  }

  pub fn join(&self, room: Arc<Uuid>, conn: QuicOrWsConn, conn_id: QuicOrWsConnId) {
    let room = self.inner.entry(room).or_insert_with(Default::default);
    room.memebers.insert(conn_id, conn);
  }

  pub fn leave(&self, room: Arc<Uuid>, conn_id: QuicOrWsConnId) {
    let room = self.inner.entry(room).or_insert_with(Default::default);
    room.memebers.remove(&conn_id);
  }
}
#[derive(Singleton, Default)]
pub struct Room {
  pub memebers: DashMap<QuicOrWsConnId, QuicOrWsConn>,
}

impl Room {
  pub async fn send(&self, conn_id: QuicOrWsConnId, pkt: Vec<u8>) {
    let pkt = Arc::new(pkt);
    let futs = FuturesUnordered::new();

    for member in &self.memebers {
      let member_id = member.key();
      let conn = member.value();
      if member_id == &conn_id {
        continue;
      }
      let pkt_clone = pkt.clone();
      match conn {
        Either::Left(conn) => {
          trace!("send to quic {}", conn_id);
          if let Ok(mut uni) = conn.open_uni().await {
            let fut = tokio::spawn(async move {
              uni.write_all(&pkt_clone).await.log();
              uni.finish().await.log();
            });
            futs.push(fut);
          } else {
            self.memebers.remove(member_id);
          };
        }
        Either::Right(conn) => {
          trace!("send to ws {}", conn_id);
          match conn.try_send(tungstenite::Message::Binary(pkt.deref().to_owned())) {
            Ok(_) => {}
            Err(TrySendError::Full(msg)) => {
              // TODO add a switch
              warn!("slow receiver of ws conn");
              conn.send(msg).await.log();
            }
            Err(TrySendError::Closed(_)) => {
              self.memebers.remove(&conn_id);
            }
          };
        }
      }
    }
    join_all(futs).await;
  }
}
