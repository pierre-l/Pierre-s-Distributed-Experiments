#[macro_use] extern crate log;
extern crate env_logger;
extern crate ring;
extern crate untrusted;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate bincode;

mod crypto;
mod transaction;

use log::LevelFilter;

fn main() {
    // Always print backtrace on panic.
    ::std::env::set_var("RUST_BACKTRACE", "1");

    env_logger::Builder::from_default_env()
        .default_format_module_path(false)
        .filter_level(LevelFilter::Info)
        .init();

    info!("Hello world.");
}