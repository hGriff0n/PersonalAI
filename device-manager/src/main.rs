
extern crate tokio;
#[macro_use] extern crate serde_json;
extern crate futures;

extern crate clap;
#[macro_use] extern crate log;
extern crate fern;
extern crate chrono;

extern crate server;

mod device;
mod serve;

// This program acts as the interaction manager for the individual device,
// Collecting and dispatching requests to the global server from modalities
// While maintaining and handling system level state/operations

use std::net::SocketAddr;

use clap::{App, Arg};
use fern::colors::{Color, ColoredLevelConfig};

// Figure out how to use futures 0.2.1 within this code
// Get working cross-device communication (move away from home ip)
    // Figure out how to handle registration/setup for modalities
    // Modify dispatch to not use hardcoded logic, instead use associated keys/etc.
// I'll also work on registering modalities with the python work

fn main() {
    let args = get_command_args();

    // Setup the logger
    let log_level = args.value_of("log-level")
        .unwrap_or("warn")
        .parse::<log::LevelFilter>()
        .unwrap();

    let log_dir = args.value_of("log-dir")
        .unwrap_or("./log");

    // TODO: Add the ability to set the log directory
    setup_logging(log_level, log_dir, args.is_present("stdio-log")).expect("Failed to initialize logging");

    trace!("Logger setup properly");

    // Setup initial listener state
    let addr = args.value_of("addr")
        .unwrap_or("127.0.0.1:6142")
        .parse::<SocketAddr>()
        .unwrap();
    let parent = "127.0.0.1:6141".parse::<SocketAddr>().ok();

    trace!("Parsed addresses");

    // TODO: Log all unmatched arguments (How do I do that?)

    // TODO: Figure out how these will interact with the new system
    // TODO: Spawn any persistent system tools and register them with the server
        // Non-persistent tasks can be spawned by the server as needed (using tokio)

    trace!("Spawned persistant tasks");

    // Spawn up the server
    serve::serve(addr, parent);
}


// Parse the command line arguments
fn get_command_args<'a>() -> clap::ArgMatches<'a> {
    App::new("Device Manager")
        .version("0.1")
        .author("Grayson Hooper <ghooper96@gmail.com>")
        .about("Manages device state and communication")
        .arg(Arg::with_name("addr")
            .long("addr")
            .value_name("IP")
            .help("Listening port and address for the manager")
            .takes_value(true))
        .arg(Arg::with_name("log-level")
            .long("log-level")
            .value_name("LEVEL")
            .help("Logging message output level")
            .takes_value(true))
        .arg(Arg::with_name("stdio-log")
            .long("stdio-log")
            .help("Control whether messages should be printed to stdout"))
        .arg(Arg::with_name("log-dir")
            .long("log-dir")
            .help("Log directory location")
            .value_name("DIR")
            .takes_value(true))
        .get_matches()
}


fn setup_logging<'a>(level: log::LevelFilter, log_dir: &'a str, io_logging: bool) -> Result<(), fern::InitError> {
    let colors_line = ColoredLevelConfig::new()
        .error(Color::Red)
        .warn(Color::Yellow)
        .info(Color::White)
        .debug(Color::White)
        .trace(Color::BrightBlack);

    let colors_level = colors_line.clone().debug(Color::Green);

    let file_logger = fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{date}][{target}:{line}][{level}] {message}",
                date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                target = record.file().unwrap_or("UNK"),
                line = record.line().unwrap_or(0),
                level = record.level(),
                message = message,
            ));
        })
        .chain(fern::log_file(format!("{}/device-manager.log", log_dir))?);

    let io_logger = fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{color_line}[{date}][{target}:{line}][{level}{color_line}] {message}\x1B[0m",
                color_line = format_args!("\x1B[{}m", colors_line.get_color(&record.level()).to_fg_str()),
                date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                target = record.file().unwrap_or("UNK"),
                line = record.line().unwrap_or(0),
                level = colors_level.color(record.level()),
                message = message,
            ));
        })
        .chain(std::io::stdout());

    let mut logger = fern::Dispatch::new()
        .level(log::LevelFilter::Warn)
        .level_for("device-manager", level)
        .level_for("device_manager", level)
        .chain(file_logger);

    if io_logging {
        logger = logger.chain(io_logger)
    }

    logger.apply()?;
    Ok(())
}

// API Documentation:
//  tokio: https://github.com/tokio-rs/tokio
//  tokio-serde-json: https://github.com/carllerche/tokio-serde-json
//  clap: https://github.com/kbknapp/clap-rs
