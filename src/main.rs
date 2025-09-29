use std::{time::Duration};

use rusb::{Context, UsbContext, Direction, TransferType};

fn main() {
    let context = Context::new().unwrap();

    let current_devices = context.devices().unwrap();
    for device in current_devices.iter() {
        // for ps4 controller we want 054c:05c4
        let desc = device.device_descriptor().unwrap();
        if desc.vendor_id() == 0x54c && desc.product_id() == 0x5c4 {
            let mut ds_endpoint= device.address();

            let config_desc = device.config_descriptor(0).unwrap();
            for interface in config_desc.interfaces() {
                for interface_desc in interface.descriptors() {
                    for endpoint_desc in interface_desc.endpoint_descriptors() {
                        if endpoint_desc.transfer_type() == TransferType::Interrupt && endpoint_desc.direction() == Direction::In {
                            ds_endpoint = endpoint_desc.address();
                        }
                    }
                }
            }

            let ds_handle = device.open().unwrap();
            let _ = ds_handle.claim_interface(0);
            let mut buf = [0u8; 64];

            loop {
                match ds_handle.read_interrupt(ds_endpoint, &mut buf, Duration::from_millis(1000)) {
                    Ok(len) => {
                        // println!("{:?}", &buf[..len]);
                        print_input(&buf[..len]);
                    }
                    Err(e) => {
                        eprintln!("Had an error reading devices: {}", e);
                        break;
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
struct Input {
    buttons: Vec<ControllerButton>,
    code: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ControllerButton {
    Null,
    UpDpad,
    RightDpad,
    DownDpad,
    LeftDpad,
    Square,
    Cross,
    Circle,
    Triangle,
    L1,
    L2,
    R1,
    R2,
    LeftJoystickPress,
    RightJoystickPress,
    Share,
    Options
}

#[derive(Debug)]
enum JoystickDirection {
    Up,
    Down,
    Center,
    Left,
    Right
}

fn determine_actions(data: u8, buttons: &mut Vec<ControllerButton>) {
    let new_data;
    match data {
        128.. => {
            buttons.push(ControllerButton::Triangle);
            new_data = data - 128;
        }
        64..128 => {
            buttons.push(ControllerButton::Circle);
            new_data = data - 64;
        }
        32..64 => {
            buttons.push(ControllerButton::Cross);
            new_data = data - 32;
        }
        16..32 => {
            buttons.push(ControllerButton::Square);
            new_data = data - 16;
        }
        ..16 => {
            // Left + Up            | -1
            // Left                 | -2
            // Left + Down          | -3
            // Down                 | -4
            // Right + Down         | -5
            // Right                | -6
            // Up + Right           | -7
            // Up                   | -8
            match data {
                9.. => panic!("Shouldn't see this: {}", data),
                8 => {}
                7 => {
                    buttons.push(ControllerButton::LeftDpad);
                    buttons.push(ControllerButton::UpDpad);
                }
                6 => {
                    buttons.push(ControllerButton::LeftDpad);
                }
                5 => {
                    buttons.push(ControllerButton::LeftDpad);
                    buttons.push(ControllerButton::DownDpad);
                }
                4 => {
                    buttons.push(ControllerButton::DownDpad);
                }
                3 => {
                    buttons.push(ControllerButton::RightDpad);
                    buttons.push(ControllerButton::DownDpad);
                }
                2 => {
                    buttons.push(ControllerButton::RightDpad);
                }
                1 => {
                    buttons.push(ControllerButton::RightDpad);
                    buttons.push(ControllerButton::UpDpad);
                }
                0 => {
                    buttons.push(ControllerButton::UpDpad);
                }
            }
            return;
        }
    }

    return determine_actions(new_data, buttons)
}

fn determine_triggers(data: u8, buttons: &mut Vec<ControllerButton>) {
    let new_data;
    match data {
        128.. => {
            buttons.push(ControllerButton::RightJoystickPress);
            new_data = data - 128;
        },
        64..128 => {
            buttons.push(ControllerButton::LeftJoystickPress);
            new_data = data - 64;
        }, 
        32..64 => {
            buttons.push(ControllerButton::Options);
            new_data = data - 32;
        }, 
        16..32 => {
            buttons.push(ControllerButton::Share);
            new_data = data - 16;
        }, 
        8..16 => {
            buttons.push(ControllerButton::R2);
            new_data = data - 8;
        }, 
        4..8 => {
            buttons.push(ControllerButton::L2);
            new_data = data - 4;
        }, 
        2..4 => {
            buttons.push(ControllerButton::R1);
            new_data = data - 2;
        }, 
        1..2 => {
            buttons.push(ControllerButton::L1);
            new_data = data - 1;
        }, 
        _ => panic!("Should not get here"),
    };

    if new_data == 0 {
        return;
    } else {
        return determine_triggers(new_data, buttons)
    }
}

fn print_input(data: &[u8]) {
    let action_buttons = process_actions(data[5]);
    let triggers = process_triggers(data[6]);
    let left_joystick_direction = process_joystick_direction(&data[1..=2]);
    let right_joystick_direction = process_joystick_direction(&data[3..=4]);

    println!("Buttons: {:?} - triggers: {:?} - left_joystick: {:?} - right_joystick: {:?}", action_buttons, triggers, left_joystick_direction, right_joystick_direction);
}

fn process_actions(action: u8) -> Input {
    if action == 8 {
        return Input {buttons: vec![ControllerButton::Null], code: 8}
    }
    let mut buttons = vec![];
    determine_actions(action, &mut buttons);

    return Input { buttons: buttons, code: action};
}

fn process_triggers(trigger: u8) -> Input {
    if trigger == 0 {
        return Input {buttons: vec![ControllerButton::Null], code: 0}
    }

    let mut buttons = vec![];
    determine_triggers(trigger, &mut buttons);
    return Input { buttons: buttons, code: trigger };
}

fn process_joystick_direction(joystick_data: &[u8]) -> (JoystickDirection, JoystickDirection) {
    let up_or_down = joystick_data[1];
    let left_or_right = joystick_data[0];

    let direction1 = match up_or_down {
        0..120 => JoystickDirection::Up,
        120..140 => JoystickDirection::Center,
        140..=255 => JoystickDirection::Down 
    };
    let direction2 = match left_or_right {
        0..120 => JoystickDirection::Left,
        120..140 => JoystickDirection::Center,
        140..=255 => JoystickDirection::Right 
    };

    return (direction1, direction2)
}