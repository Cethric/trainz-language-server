use log::trace;
use shadow_rs::shadow;

shadow!(build);

pub fn setup_logger(level: Option<log::LevelFilter>) {
    let mut builder = env_logger::builder();
    builder.format_timestamp_millis();
    if let Some(level) = level {
        builder.filter_level(level);
    }
    builder.format_target(true);
    builder.init();

    trace!("Logger initialized")
}
