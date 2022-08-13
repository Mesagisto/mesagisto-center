use arcstr::ArcStr;
use color_eyre::eyre::{Error, Result};

#[config_derive]
#[derive(AutomaticConfig)]
#[location = "config/center.yml"]
pub struct Config {
  #[educe(Default = false)]
  pub enable: bool,
  pub server: ServerConfig,
  pub tls: TlsConfig,
}

#[config_derive]
pub struct TlsConfig {
  #[educe(Default = true)]
  pub enable_for_ws: bool,
  #[educe(Default = "/path/to/cert")]
  pub cert: ArcStr,
  #[educe(Default = "/path/to/key")]
  pub key: String,
}

#[config_derive]
pub struct ServerConfig {
  #[educe(Default = "0.0.0.0:6996")]
  pub ws: ArcStr,
  #[educe(Default = "0.0.0.0:6996")]
  pub quic: String,
}
