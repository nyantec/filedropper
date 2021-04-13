mod error;
mod server;

pub use error::*;
pub use server::FileDropper;

use byte_unit::Byte;
use getopts::Options;
use log::error;

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
    opts.optopt(
        "s",
        "max_size",
        "Maximum allowed request body size",
        "BYTES",
    );
    opts.optopt("b", "before_text", "Text shown before the upload", "TEXT");
    opts.optopt("e", "error_text", "Text shown if an error occurs", "TEXT");
    opts.optopt(
        "t",
        "success_text",
        "Text shown after a successful upload",
        "TEXT",
    );
    opts.optflag("h", "help", "Display this help text and exit");

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

    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }

    let listen_addr = matches
        .opt_str("l")
        .as_deref()
        .unwrap_or("127.0.0.1:3000")
        .parse()
        .unwrap();

    let max_size = Byte::from_str(matches.opt_str("s").as_deref().unwrap_or("100M"))
        .unwrap()
        .get_bytes();

    let before_text = matches.opt_str("b").unwrap_or("".to_string());

    let error_text = matches
        .opt_str("e")
        .unwrap_or("Error: ${error}".to_string());

    let success_text = matches
        .opt_str("t")
        .unwrap_or("Upload successful!".to_string());

    let output = if !matches.free.is_empty() {
        matches.free[0].clone()
    } else {
        print_usage(&program, opts);
        return;
    };

    env_logger::init();

    let html = include_str!("index.html")
        .replace("@beforeText@", &before_text)
        .replace("@errorText@", &error_text)
        .replace("@successText@", &success_text);

    FileDropper::new(listen_addr, output, max_size, html)
        .serve()
        .unwrap_or_else(|e| {
            error!("Error: {}, exiting", e.to_string());
            std::process::exit(1);
        });
}
