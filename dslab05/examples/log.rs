use log::{debug, error, info, trace, warn, LevelFilter};
use simplelog::{Config, TermLogger, TerminalMode};

fn main() {
    TermLogger::init(LevelFilter::Trace, Config::default(), TerminalMode::Mixed)
        .expect("No interactive terminal");

    error!("Bright red error");
    warn!("Warning, yellow");
    info!("Calm blue info");
    debug!("Less important message, not needed in production");
    trace!("Last resort level of logging for debug, you can produce huge volumes of those in your code");
    // Logging to a different target.
    info!(target: "a_target", "a message to target");
}
