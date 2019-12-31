#[cfg(windows)]
#[path = "main_win32.rs"]
mod platform;

fn main() {
    platform::main();
}
