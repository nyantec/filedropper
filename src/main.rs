mod error;
mod server;

pub use error::*;
pub use server::FileDropper;

use getopts::Options;
use log::error;
use std::net::SocketAddr;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} OUTPUT [options]", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optopt(
        "l",
        "listen",
        "Listen on the specified sockaddr",
        "ADDR:PORT",
    );
    opts.optflag("h", "help", "Display this help text and exit");
    opts.optflag(
        "v",
        "version",
        "Display the version of this binary and exit",
    );

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            panic!("{}", f)
        }
    };

    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }

    let listen_addr: SocketAddr = matches
        .opt_str("l")
        .as_deref()
        .unwrap_or("127.0.0.1:3000")
        .parse()
        .unwrap();

    let output = if !matches.free.is_empty() {
        matches.free[0].clone()
    } else {
        print_usage(&program, opts);
        return;
    };

    env_logger::init();

    FileDropper::new(listen_addr, output)
        .serve()
        .unwrap_or_else(|e| {
            error!("Error: {}, exiting", e.to_string());
            std::process::exit(1);
        });
}
