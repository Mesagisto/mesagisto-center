use chrono::{Local, Offset, TimeZone};
use color_eyre::eyre::Result;
use tracing::Level;
use tracing_error::ErrorLayer;
use tracing_subscriber::{layer::SubscriberExt, prelude::*};

pub(crate) async fn init() -> Result<()> {
  let filter = tracing_subscriber::filter::Targets::new()
    .with_target("mesagisto_center", Level::TRACE)
    .with_default(Level::WARN);

  let registry = tracing_subscriber::registry();

  registry
    .with(filter)
    .with(ErrorLayer::default())
    .with(
      tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_timer(tracing_subscriber::fmt::time::OffsetTime::new(
          time::UtcOffset::from_whole_seconds(
            Local.timestamp(0, 0).offset().fix().local_minus_utc(),
          )
          .unwrap_or(time::UtcOffset::UTC),
          time::macros::format_description!(
            "[year repr:last_two]-[month]-[day] [hour]:[minute]:[second]"
          ),
        )),
    )
    .try_init()?;
  Ok(())
}
