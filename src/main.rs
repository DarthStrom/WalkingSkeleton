mod game;
#[cfg(target_os = "windows")]
#[path = "main_win32.rs"]
mod platform;

#[macro_use]
extern crate log;

fn main() {
    env_logger::init();
    // log levels: error, warn, info, debug, trace
    info!("starting up... log level: {}", log::max_level());

    platform::main();
}
