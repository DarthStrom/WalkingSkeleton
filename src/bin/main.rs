#[path = "../common.rs"]
pub mod common;

#[cfg(target_os = "windows")]
#[path = "../os/win32.rs"]
mod os;

#[macro_use]
extern crate log;

fn main() {
    // log levels: error, warn, info, debug, trace
    info!("starting up... log level: {}", log::max_level());

    os::main();
}
