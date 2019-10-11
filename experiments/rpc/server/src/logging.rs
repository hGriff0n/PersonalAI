
use std;
use std::io::{Error, ErrorKind};

use chrono;
use clap;
use fern;
use fern::colors::{Color, ColoredLevelConfig};
use log;

// Extract the "module" arguments and spawn the logging instance
// TODO: Convert to using the 'errors' module
pub fn launch<'a>(args: &'a clap::ArgMatches) -> Result<(), fern::InitError> {
    let level = args.value_of("log_level")
        .unwrap_or("warn")
        .parse::<log::LevelFilter>()
        .map_err(|_err| fern::InitError::Io(Error::new(ErrorKind::NotFound, "Log level not found in configuration")))?;

    let log_dir = args.value_of("log_dir")
        .unwrap_or("./log");
    {
        let log_dir_path = std::path::Path::new(&log_dir);
        if !log_dir_path.exists() {
            std::fs::create_dir(log_dir_path)?;
        }
    }

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

    let mut logger = fern::Dispatch::new()
        .level(log::LevelFilter::Warn)
        .level_for("device-manager", level)
        .level_for("device_manager", level)
        .chain(file_logger);

    // Add an additional log "fan" to print stuff to stdout if requested
    if args.is_present("stdio_log") {
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

        logger = logger.chain(io_logger)
    }

    logger.apply()?;
    Ok(())
}

// Register logging specific command line arguments with clap::App
pub fn add_args<'a, 'b>(app: clap::App<'a, 'b>) -> clap::App<'a, 'b> {
    use clap::Arg;

    app.arg(Arg::with_name("log_level")
            .long("log_level")
            .value_name("LEVEL")
            .help("Logging message output level")
            .takes_value(true))
        .arg(Arg::with_name("stdio_log")
            .long("stdio_log")
            .help("Control whether messages should be printed to stdout"))
        .arg(Arg::with_name("log_dir")
            .long("log_dir")
            .help("Log directory location")
            .value_name("DIR")
            .takes_value(true))
}
