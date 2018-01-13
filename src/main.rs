extern crate arrayvec;
extern crate env_logger;
#[macro_use]
extern crate failure;
extern crate i2cdev;
#[macro_use]
extern crate log;

mod thunder_borg;

use std::process;
use std::env;
use env_logger::LogBuilder;
use log::{LogLevelFilter, LogRecord};
use failure::Error;
use thunder_borg::Controller;
use std::thread;
use std::time::Duration;

fn run() -> Result<(), Error> {
    let mut controller = Controller::new()?;
    let mut num_iter = 0;
    while num_iter < 2 {
        controller.set_motors(0.1)?;
        thread::sleep(Duration::from_millis(100));
        info!(
            "A fault: {} | B fault: {} | Battery voltage: {:.2}V",
            controller.get_drive_fault_a()?,
            controller.get_drive_fault_b()?,
            controller.get_battery_voltage()?
        );

        controller.set_motors(0.8)?;
        thread::sleep(Duration::from_millis(1800));
        controller.set_motors(0.1)?;
        thread::sleep(Duration::from_millis(100));
        controller.set_motors(0.0)?;

        thread::sleep(Duration::from_millis(5000));

        controller.set_motors(-0.1)?;
        thread::sleep(Duration::from_millis(100));
        controller.set_motors(-0.8)?;
        thread::sleep(Duration::from_millis(1800));
        controller.set_motors(-0.1)?;
        thread::sleep(Duration::from_millis(100));
        controller.set_motors(0.0)?;

        thread::sleep(Duration::from_millis(3000));

        num_iter += 1;
        info!(
            "A fault: {} | B fault: {} | Battery voltage: {:.2}V",
            controller.get_drive_fault_a()?,
            controller.get_drive_fault_b()?,
            controller.get_battery_voltage()?
        );
    }
    controller.stop()?;

    Ok(())
}

fn init_env_logger() -> Result<(), Error> {
    let format = |record: &LogRecord| format!("[{}]: {}", record.level(), record.args());

    let mut builder = LogBuilder::new();
    builder.format(format).filter(None, LogLevelFilter::Info);

    if env::var("RUST_LOG").is_ok() {
        builder.parse(&env::var("RUST_LOG").unwrap());
    }

    builder.init()?;
    Ok(())
}

fn exit_with_error(error: &Error) -> ! {
    error!("Fatal error: {} {}", error.cause(), error.backtrace());
    process::exit(1);
}

fn main() {
    if let Err(error) = init_env_logger() {
        println!(
            "Could not initialize logger, exiting: {} {}",
            error.cause(),
            error.backtrace()
        );
        process::exit(1);
    }
    if let Err(ref error) = run() {
        exit_with_error(error);
    }
}
