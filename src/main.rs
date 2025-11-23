use std::{time::Duration};
use std::thread;
use std::sync::mpsc::{self, Sender};

use rusb::{Context, UsbContext, Direction, TransferType};

fn main() {
    let (tx, rx) = mpsc::channel();
    thread::spawn(|| {
        if let Err(e) = controller_listener(tx) {
            eprintln!("USB thread error: {}", e);
        } 
    });

    loop {
        while let Ok(buttons) = rx.try_recv() {
            println!("{:?}", buttons);
            for button in buttons {
                match button {
                    ControllerButton::Cross => println!("Jumping!"),
                    _ => {}
                }
            }
        }
    }
}

fn controller_listener(thread_tx: Sender<Vec<ControllerButton>>) -> Result<(), Box<dyn std::error::Error>> {
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
                        // print_input(&buf[..len]);
                        send_input(&buf[..len], &thread_tx); 
                    }
                    Err(e) => {
                        eprintln!("Had an error reading devices: {}", e);
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct Input {
    buttons: Vec<ControllerButton>,
    code: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ControllerButton {
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
    // Buttons
    if (data & (1 << 7)) > 0 {
        buttons.push(ControllerButton::Triangle);
    }
    if (data & (1 << 6)) > 0 {
        buttons.push(ControllerButton::Circle);
    }
    if (data & (1 << 5)) > 0 {
        buttons.push(ControllerButton::Cross);
    }
    if (data & (1 << 4)) > 0 {
        buttons.push(ControllerButton::Square);
    }

    // Dpad
    let first_four_bits = 0x0F & data;
    if first_four_bits == 8 {
        return;
    }

    if first_four_bits == 7 {
        buttons.push(ControllerButton::LeftDpad);
        buttons.push(ControllerButton::UpDpad);
    } else if first_four_bits == 6 {
        buttons.push(ControllerButton::LeftDpad);
    } else if first_four_bits == 5 {
        buttons.push(ControllerButton::LeftDpad);
        buttons.push(ControllerButton::DownDpad);
    } else if first_four_bits == 4 {
        buttons.push(ControllerButton::DownDpad);
    } else if first_four_bits == 3 {
        buttons.push(ControllerButton::RightDpad);
        buttons.push(ControllerButton::DownDpad);
    } else if first_four_bits == 2 {
        buttons.push(ControllerButton::RightDpad);
    } else if first_four_bits == 1 {
        buttons.push(ControllerButton::RightDpad);
        buttons.push(ControllerButton::UpDpad);
    } else if first_four_bits == 0 {
        buttons.push(ControllerButton::UpDpad);
    }
    return;
}

fn determine_triggers(data: u8, buttons: &mut Vec<ControllerButton>) {
    if (data & (1 << 7)) > 0 {
            buttons.push(ControllerButton::RightJoystickPress);
    }
    if (data & (1 << 6)) > 0 {
            buttons.push(ControllerButton::LeftJoystickPress);
    }
    if (data & (1 << 5)) > 0 {
            buttons.push(ControllerButton::Options);
    }
    if (data & (1 << 4)) > 0 {
            buttons.push(ControllerButton::Share);
    }
    if (data & (1 << 3)) > 0 {
            buttons.push(ControllerButton::R2);
    }
    if (data & (1 << 2)) > 0 {
            buttons.push(ControllerButton::L2);
    }
    if (data & (1 << 1)) > 0 {
            buttons.push(ControllerButton::R1);
    }
    if (data & (1 << 0)) > 0 {
            buttons.push(ControllerButton::L1);
    }
    return; 
}

// fn print_input(data: &[u8]) {
//     let action_buttons = process_actions(data[5]);
//     let triggers = process_triggers(data[6]);
//     let left_joystick_direction = process_joystick_direction(&data[1..=2]);
//     let right_joystick_direction = process_joystick_direction(&data[3..=4]);

//     println!("Buttons: {:?} - triggers: {:?} - left_joystick: {:?} - right_joystick: {:?}", action_buttons, triggers, left_joystick_direction, right_joystick_direction);
// }

fn send_input(data: &[u8], thread_tx: &Sender<Vec<ControllerButton>>) {
    let mut action_buttons = process_actions(data[5]);
    let mut triggers = process_triggers(data[6]);
    // let left_joystick_direction = process_joystick_direction(&data[1..=2]);
    // let right_joystick_direction = process_joystick_direction(&data[3..=4]);


    action_buttons.append(&mut triggers);
    if action_buttons.len() > 0 {
        thread_tx.send(action_buttons).unwrap()
    }
}

fn process_actions(action: u8) -> Vec<ControllerButton> {
    if action == 8 {
        return vec![];
    }
    let mut buttons = vec![];
    determine_actions(action, &mut buttons);
    return buttons
}

fn process_triggers(trigger: u8) -> Vec<ControllerButton> {
    if trigger == 0 {
        return vec![];
    }

    let mut buttons = vec![];
    determine_triggers(trigger, &mut buttons);
    return buttons;
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