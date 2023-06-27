use crate::arch::x86_64::{inb, outb};

bitflags::bitflags! {
    struct StatusRegisterFlags: u8 {
        const OUTPUT_BUFFER_FULL = 1 << 0;
        const INPUT_BUFFER_FULL = 1 << 1;
        const SYSTEM_FLAG = 1 << 2;
        const INPUT_BUFFER_FOR_CONTROLLER = 1 << 3;
        const UNKNOWN1 = 1 << 4;
        const UNKNOWN2 = 1 << 5;
        const TIMEOUT_ERROR = 1 << 6;
        const PARITY_ERROR = 1 << 7;
    }

    struct ConfigByteFlags: u8 {
        const FIRST_PORT_INTERRUPT_ENABLED = 1 << 0;
        const SECOND_PORT_INTERRUPT_ENABLED = 1 << 1;
        const SYSTEM_FLAG = 1 << 2;
        const ZERO1 = 1 << 3;
        const FIRST_PORT_CLOCK_DISABLED = 1 << 4;
        const SECOND_PORT_CLOCK_DISABLED = 1 << 5;
        const FIRST_PORT_TRANSLATION = 1 << 6;
        const ZERO2 = 1 << 7;
    }

    struct ControllerOutputFlags: u8 {
        const SYSTEM_RESET = 1 << 0; // always 1
        const A20_GATE = 1 << 1;
        const SECOND_PORT_CLOCK = 1 << 2;
        const SECOND_PORT_DATA = 1 << 3;
        const OUTPUT_BUFFER_FULL_FIRST_PORT = 1 << 4;
        const OUTPUT_BUFFER_FULL_SECOND_PORT = 1 << 5;
        const FIRST_PORT_CLOCK = 1 << 6;
        const FIRST_PORT_DATA = 1 << 7;
    }
}

#[derive(Debug)]
pub enum PS2ControllerError {
    ConfigFileReadFailed,
    SelfTestFailed,
    DataBufferWriteFailed,
}

const DATA_REGISTER_PORT: u16 = 0x60;
const STATUS_REGISTER_PORT: u16 = 0x64;
const COMMAND_REGISTER_PORT: u16 = 0x64;

const CMD_READ_CFG_BYTE: u8 = 0x20;
const CMD_WRITE_CFG_BYTE: u8 = 0x60;

const CMD_TEST_CONTROLLER: u8 = 0xAA;

const CMD_TEST_FIRST_PORT: u8 = 0xAB;
const CMD_TEST_SECOND_PORT: u8 = 0xA9;

const CMD_ENABLE_FIRST_PORT: u8 = 0xAE;
const CMD_DISABLE_FIRST_PORT: u8 = 0xAD;

const CMD_ENABLE_SECOND_PORT: u8 = 0xA8;
const CMD_DISABLE_SECOND_PORT: u8 = 0xA7;

const CMD_NEXT_BYTE_SECOND_PORT: u8 = 0xD4;

const SELF_TEST_SUCCESS: u8 = 0x55;

const DEVICE_CMD_RESET: u8 = 0xFF;
const DEVICE_RESET_SUCCESS: u8 = 0xFA;
const DEVICE_RESET_FAILURE: u8 = 0xFC;

fn read_status() -> StatusRegisterFlags {
    let status = inb(STATUS_REGISTER_PORT);
    StatusRegisterFlags::from_bits(status).unwrap()
}

fn read_config_byte() -> Result<ConfigByteFlags, PS2ControllerError> {
    match send_command_response(CMD_READ_CFG_BYTE) {
        Ok(val) => Ok(ConfigByteFlags::from_bits(val).unwrap()),
        Err(_) => Err(PS2ControllerError::ConfigFileReadFailed),
    }
}

fn write_config_byte(cfg: ConfigByteFlags) -> Result<(), PS2ControllerError> {
    send_command(CMD_WRITE_CFG_BYTE);
    write_data_buffer(cfg.bits)
}

pub fn read_data_buffer() -> Result<u8, ()> {
    if !wait_until_output_buffer_full() {
        return Err(());
    }

    let val = inb(DATA_REGISTER_PORT);
    Ok(val)
}

fn write_data_buffer(val: u8) -> Result<(), PS2ControllerError> {
    if wait_until_output_buffer_empty() {
        outb(DATA_REGISTER_PORT, val);
        Ok(())
    } else {
        Err(PS2ControllerError::DataBufferWriteFailed)
    }
}

fn wait_until_output_buffer_full() -> bool {
    const TIMEOUT: usize = 100000;
    for _ in 0..TIMEOUT {
        let status = read_status();
        if status.contains(StatusRegisterFlags::OUTPUT_BUFFER_FULL) {
            return true;
        }
    }

    false
}

fn wait_until_output_buffer_empty() -> bool {
    const TIMEOUT: usize = 10000;
    for _ in 0..TIMEOUT {
        let status = read_status();
        if !status.contains(StatusRegisterFlags::OUTPUT_BUFFER_FULL) {
            return true;
        }
    }

    false
}

fn send_command(cmd: u8) {
    outb(COMMAND_REGISTER_PORT, cmd);
}

fn send_command_response(cmd: u8) -> Result<u8, ()> {
    outb(COMMAND_REGISTER_PORT, cmd);
    read_data_buffer()
}

fn write_data_first_port(val: u8) -> Result<(), PS2ControllerError> {
    write_data_buffer(val)
}

fn write_data_second_port(val: u8) -> Result<(), PS2ControllerError> {
    send_command(CMD_NEXT_BYTE_SECOND_PORT);
    write_data_buffer(val)
}

pub fn init() -> Result<(bool, bool), PS2ControllerError> {
    // disable both channels
    send_command(CMD_DISABLE_FIRST_PORT);
    send_command(CMD_DISABLE_SECOND_PORT);

    // discard data stuck in data buffer
    inb(DATA_REGISTER_PORT);

    let mut config_byte = read_config_byte()?;

    // disable interrupts and translation
    config_byte.remove(ConfigByteFlags::FIRST_PORT_INTERRUPT_ENABLED);
    config_byte.remove(ConfigByteFlags::SECOND_PORT_INTERRUPT_ENABLED);
    config_byte.remove(ConfigByteFlags::FIRST_PORT_TRANSLATION);

    // write config byte
    write_config_byte(config_byte)?;

    match send_command_response(CMD_TEST_CONTROLLER) {
        Ok(res) => {
            if res != SELF_TEST_SUCCESS {
                return Err(PS2ControllerError::SelfTestFailed);
            }
        }
        Err(_) => return Err(PS2ControllerError::SelfTestFailed),
    };

    // rewrite config byte because the self test sometimes resets the controller
    write_config_byte(config_byte)?;

    let dual_channel = {
        send_command(CMD_ENABLE_SECOND_PORT);
        let config_byte = read_config_byte()?;

        !config_byte.contains(ConfigByteFlags::SECOND_PORT_CLOCK_DISABLED)
    };

    if dual_channel {
        send_command(CMD_DISABLE_SECOND_PORT);
    }

    let (mut first_port_working, mut second_port_working) = (
        send_command_response(CMD_TEST_FIRST_PORT).map_or(false, |n| n == 0),
        dual_channel && send_command_response(CMD_TEST_SECOND_PORT).map_or(false, |n| n == 0),
    );

    if first_port_working {
        send_command(CMD_ENABLE_FIRST_PORT);
        config_byte.remove(ConfigByteFlags::FIRST_PORT_CLOCK_DISABLED);
        config_byte.insert(ConfigByteFlags::FIRST_PORT_INTERRUPT_ENABLED);
    }

    if second_port_working {
        send_command(CMD_ENABLE_SECOND_PORT);
        config_byte.remove(ConfigByteFlags::SECOND_PORT_CLOCK_DISABLED);
        config_byte.insert(ConfigByteFlags::SECOND_PORT_INTERRUPT_ENABLED);
    }

    // reset devices
    first_port_working &= write_data_first_port(DEVICE_CMD_RESET).map_or(false, |_| {
        let res = read_data_buffer().unwrap_or(DEVICE_RESET_FAILURE);
        res == DEVICE_RESET_SUCCESS
    });

    second_port_working &= write_data_second_port(DEVICE_CMD_RESET).map_or(false, |_| {
        send_command(CMD_NEXT_BYTE_SECOND_PORT);
        let res = read_data_buffer().unwrap_or(DEVICE_RESET_FAILURE);
        res == DEVICE_RESET_SUCCESS
    });

    read_data_buffer().unwrap();

    config_byte.insert(ConfigByteFlags::FIRST_PORT_TRANSLATION);

    // enable interrupts
    write_config_byte(config_byte)?;

    // TODO: maybe disable interrupts for a channel that failed to reset

    Ok((first_port_working, second_port_working))
}
