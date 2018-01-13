use i2cdev::core::*;
use i2cdev::linux::LinuxI2CDevice;
use failure::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use arrayvec::ArrayVec;

#[derive(Debug, Fail)]
enum ControllerError {
    #[fail(display = "error while running command {}", command)] CommandError { command: Command },
}

pub struct Controller {
    dev: LinuxI2CDevice,
}

impl Controller {
    pub fn new() -> Result<Self, Error> {
        info!(
            "Pinging ThunderBorg at i2c bus {} address 0x{:x}",
            1, THUNDERBORG_SLAVE_ADDR
        );
        let mut controller = Controller {
            dev: LinuxI2CDevice::new("/dev/i2c-1", THUNDERBORG_SLAVE_ADDR)?,
        };

        let response = controller.command_with_response(Command::GetId)?;
        if response[1] == THUNDERBORG_ID {
            info!("ThunderBorg chip found. ");
        } else {
            panic!("Found chip with different id");
        }
        Ok(controller)
    }

    pub fn set_led(&mut self, red: u8, green: u8, blue: u8) -> Result<(), Error> {
        self.command(Command::SetLed, &[red, green, blue])
    }

    pub fn set_motors(&mut self, power: f32) -> Result<(), Error> {
        self.motor_command(Command::SetMotorsForward, Command::SetMotorsReverse, power)
    }

    pub fn set_motor_a(&mut self, power: f32) -> Result<(), Error> {
        self.motor_command(Command::SetMotorAForward, Command::SetMotorAReverse, power)
    }

    pub fn set_motor_b(&mut self, power: f32) -> Result<(), Error> {
        self.motor_command(Command::SetMotorBForward, Command::SetMotorBReverse, power)
    }

    pub fn get_drive_fault_a(&mut self) -> Result<bool, Error> {
        let response = self.command_with_response(Command::GetDriveFaultFlagA)?;
        Ok(response[1] != I2C_VALUE_OFF)
    }

    pub fn get_drive_fault_b(&mut self) -> Result<bool, Error> {
        let response = self.command_with_response(Command::GetDriveFaultFlagB)?;
        Ok(response[1] != I2C_VALUE_OFF)
    }

    pub fn stop(&mut self) -> Result<(), Error> {
        self.command(Command::AllOff, &[0])
    }

    pub fn get_battery_voltage(&mut self) -> Result<f32, Error> {
        let voltage_bytes = self.command_with_response(Command::GetBatteryVoltage)?;
        let raw_voltage = ((voltage_bytes[1] as u16) << 8) + (voltage_bytes[2] as u16);
        Ok((raw_voltage as f32) / COMMAND_ANALOG_MAX * VOLTAGE_PIN_MAX + VOLTAGE_PIN_CORRECTION)
    }

    fn motor_command(
        &mut self,
        forward_command: Command,
        reverse_command: Command,
        power: f32,
    ) -> Result<(), Error> {
        let power = clamp_motor_power(power);
        let power_bytes = &[motor_power_to_byte(power)];
        if power < 0.0 {
            self.command(reverse_command, power_bytes)?;
        } else {
            self.command(forward_command, power_bytes)?;
        }
        Ok(())
    }

    fn command_with_response(&mut self, command: Command) -> Result<I2CResponse, Error> {
        let mut attempt = I2C_COMMAND_NUM_ATTEMPTS;
        while attempt > 0 {
            debug!("Writing command {} to i2c bus", command);
            let wire_command = command.to_wire();
            self.dev.smbus_write_byte(wire_command)?;

            let mut response = [0u8; I2C_MAX_LEN];
            self.dev.read(&mut response)?;
            debug!("Read bytes from i2c bus: {:?}", response);
            if response[0] != wire_command {
                attempt -= 1;
                info!("Retrying (read {})", response[0]);
            } else {
                return Ok(response);
            }
        }
        error!("Failed to run command {}", command);
        Err((ControllerError::CommandError { command }).into())
    }

    fn command(&mut self, command: Command, data: &[u8]) -> Result<(), Error> {
        debug!("Writing command {} {:?} to bus", command, data);
        let mut command_bytes = ArrayVec::<[u8; I2C_MAX_LEN]>::new();
        command_bytes.push(command.to_wire());
        command_bytes.extend(data.iter().map(|x| *x));
        self.dev.write(&command_bytes)?;
        Ok(())
    }
}

impl Drop for Controller {
    fn drop(&mut self) {
        info!("Destroying a ThunderBorg `Controller`. Ensuring engines are stopped...");
        if let Err(error) = self.stop() {
            error!(
                "Could not run `stop()` when destroying the controller. Error: {}",
                error
            );
            error!("Motors may still be running!");
        }
    }
}

#[derive(Debug)]
enum Command {
    /// Set the colour of the ThunderBorg LED
    SetLed,
    /// Get the colour of the ThunderBorg LED
    GetLed,
    /// Set motor A PWM rate in a forwards direction
    SetMotorAForward,
    /// Set motor A PWM rate in a reverse direction
    SetMotorAReverse,
    /// Get motor A direction and PWM rate
    GetMotorA,
    /// Set motor B PWM rate in a forwards direction
    SetMotorBForward,
    /// Set motor B PWM rate in a reverse direction
    SetMotorBReverse,
    /// Get motor B direction and PWM rate
    GetMotorB,
    ///  Switch everything off
    AllOff,
    /// Get the drive fault flag for motor A, indicates faults such as
    /// short-circuits and under voltage
    GetDriveFaultFlagA,
    /// Get the drive fault flag for motor B, indicates faults such as
    /// short-circuits and under voltage
    GetDriveFaultFlagB,
    /// Set all motors PWM rate in a forwards direction
    SetMotorsForward,
    /// Set all motors PWM rate in a reverse direction
    SetMotorsReverse,
    /// Get the battery voltage reading
    GetBatteryVoltage,
    /// Get the board identifier
    GetId,
}

impl Display for Command {
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        let pretty_name = match *self {
            Command::SetLed => "SetLed",
            Command::GetLed => "GetLed",
            Command::SetMotorAForward => "SetMotorAForward",
            Command::SetMotorAReverse => "SetMotorAReverse",
            Command::GetMotorA => "GetMotorA",
            Command::SetMotorBForward => "GetMotorBForward",
            Command::SetMotorBReverse => "GetMotorBReverse",
            Command::GetMotorB => "GetMotorB",
            Command::AllOff => "AllOff",
            Command::GetDriveFaultFlagA => "GetDriveFaultFlagA",
            Command::GetDriveFaultFlagB => "GetDriveFaultFlagB",
            Command::SetMotorsForward => "SetMotorsForward",
            Command::SetMotorsReverse => "SetMotorsReverse",
            Command::GetBatteryVoltage => "GetBatteryVoltage",
            Command::GetId => "GetId",
        };
        write!(formatter, "{} (0x{:x})", pretty_name, self.to_wire())
    }
}

impl Command {
    #[inline]
    fn to_wire(&self) -> u8 {
        match *self {
            Command::SetLed => 1,
            Command::GetLed => 2,
            Command::SetMotorAForward => 8,
            Command::SetMotorAReverse => 9,
            Command::GetMotorA => 10,
            Command::SetMotorBForward => 11,
            Command::SetMotorBReverse => 12,
            Command::GetMotorB => 13,
            Command::AllOff => 14,
            Command::GetDriveFaultFlagA => 15,
            Command::GetDriveFaultFlagB => 16,
            Command::SetMotorsForward => 17,
            Command::SetMotorsReverse => 18,
            Command::GetBatteryVoltage => 21,
            Command::GetId => 0x99,
        }
    }
}

type I2CResponse = [u8; I2C_MAX_LEN];

#[inline]
fn clamp_motor_power(value: f32) -> f32 {
    if value > 1.0 {
        1.0
    } else if value < -1.0 {
        -1.0
    } else {
        value
    }
}

#[inline]
fn motor_power_to_byte(value: f32) -> u8 {
    assert!(-1.0 <= value && value <= 1.0);
    (value.abs() * 255.0) as u8
}

const I2C_VALUE_ON: u8 = 1; // I2C value representing on
const I2C_VALUE_OFF: u8 = 0; // I2C value representing off
const I2C_COMMAND_NUM_ATTEMPTS: usize = 3;
const I2C_MAX_LEN: usize = 6;
const THUNDERBORG_ID: u8 = 0x15;
const THUNDERBORG_SLAVE_ADDR: u16 = 0x15;

// Maximum value for analog readings
const COMMAND_ANALOG_MAX: f32 = 0x3FF as f32;

// Maximum voltage from the analog voltage monitoring pin
const VOLTAGE_PIN_MAX: f32 = 36.3;

// Correction value for the analog voltage monitoring pin
const VOLTAGE_PIN_CORRECTION: f32 = 0.0;
