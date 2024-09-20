use std::io;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

mod event_handler;
use can_dbc::DBC;
use event_handler::{CanHandler, DBCFile, DebugHandler, Init, PacketFilter};
#[cfg(target_os = "windows")]
use pcan_basic::bus::UsbBus;
#[cfg(target_os = "linux")]
use privilege_rs::privilege_request;
#[cfg(target_os = "windows")]
use slint::Model;
use slint::SharedString;
#[cfg(target_os = "windows")]
use winapi::um::wincon::FreeConsole;

slint::include_modules!();

#[tokio::main]
async fn main() -> io::Result<()> {
    #[cfg(target_os = "linux")]
    if privilege_request()? == privilege_rs::Privilege::User {
        println!("Failed to request the privilege");
        std::process::exit(0);
    }
    let ui = AppWindow::new().unwrap();

    let (tx, rx) = mpsc::channel::<DBC>();
    // Wrap `rx` in an Arc<Mutex<>> so it can be shared safely across threads
    let rx = Arc::new(Mutex::new(rx));
    #[cfg(target_os = "windows")]
    unsafe {
        FreeConsole(); // This detaches the console from the application
    }

    // Find available socket CAN
    let ui_handle = ui.as_weak();
    tokio::spawn(async move {
        let init_event = Init {
            ui_handle: &ui_handle,
        };
        init_event.run();
    });

    let (start_tx_1, start_rx_1) = mpsc::channel();
    let (start_tx_2, start_rx_2) = mpsc::channel();

    // Handle start event
    let ui_handle = ui.as_weak();
    ui.on_start(move |_name, _index, bitrate| {
        #[cfg(target_os = "linux")]
        {
            let ui = ui_handle.unwrap();
            if _name.is_empty() {
                ui.set_init_string(SharedString::from("No device found!!!"));
            } else {
                ui.set_is_init(true);
                let _ = start_tx_1.send((_name.clone(), bitrate.clone()));
                let _ = start_tx_2.send((_name, bitrate));
            }
        }
        #[cfg(target_os = "windows")]
        {
            let ui = ui_handle.unwrap();
            let get_device_handle = match ui.get_can_sockets().index.row_data(_index as usize) {
                Some(device) => device,
                None => {
                    ui.set_init_string(SharedString::from("No device found!!!"));
                    return;
                }
            };
            let usb_can = UsbBus::try_from(get_device_handle as u16).unwrap();
            let _ = start_tx_1.send((usb_can, bitrate.clone()));
            let _ = start_tx_2.send((usb_can, bitrate));
            ui.set_is_init(true);
        }
    });

    let (can_tx, can_rx) = mpsc::channel();
    let ui_handle = ui.as_weak();
    tokio::spawn(async move {
        if let Ok((can_if, bitrate)) = start_rx_1.recv() {
            let mut can_handler = CanHandler {
                #[cfg(target_os = "windows")]
                iface: can_if,
                #[cfg(target_os = "linux")]
                iface: &can_if,
                ui_handle: &ui_handle,
                mspc_rx: &rx,
                bitrate: bitrate.to_string(),
                dbc: None,
                can_tx,
            };
            loop {
                can_handler.process_can_messages();
            }
        }
    });

    let ui_handle = ui.as_weak();
    tokio::spawn(async move {
        if let Ok((_can_if, bitrate)) = start_rx_2.recv() {
            let mut can_handler = DebugHandler {
                ui_handle: &ui_handle,
                bitrate: bitrate.to_string(),
                filter: (0, 0xFFFFFFFF),
                can_rx,
            };
            loop {
                can_handler.run();
            }
        }
    });

    // Handle open file event
    let ui_handle = ui.as_weak();
    ui.on_open_dbc_file(move || {
        let dbc_handle = DBCFile {
            ui_handle: &ui_handle,
            mspc_tx: &tx.clone(),
        };

        dbc_handle.process_dbc_file();
    });

    // Handle filter page
    let ui_handle = ui.as_weak();
    ui.on_filter_id(move |filter, is_check| {
        let packet_filter = PacketFilter {
            ui_handle: &ui_handle,
            filter,
            is_check,
        };
        packet_filter.process_filter();
    });

    ui.window().on_close_requested(|| {
        println!("Closing the application...");
        std::process::exit(0);
    });

    ui.on_can_id_check_string(move |is_extended, can_id| is_valid_can_id(is_extended, &can_id));

    ui.on_can_data_check_string(move |can_data| is_valid_can_data(&can_data));

    ui.run().unwrap();
    Ok(())
}

fn is_valid_can_id(is_extended: bool, can_id: &str) -> bool {
    // Try to parse the string as a hex number
    match u32::from_str_radix(can_id, 16) {
        Ok(id) => {
            if is_extended {
                id <= 0x1FFFFFFF // Extended CAN ID (29-bit max)
            } else {
                id <= 0x7FF // Standard CAN ID (11-bit max)
            }
        }
        Err(_) => false, // If parsing fails, it's not a valid hex string
    }
}

fn is_valid_can_data(can_data: &str) -> bool {
    // CAN data is valid if it's a hex string of even length up to 16 characters (8 bytes)
    if can_data.len() % 2 != 0 || can_data.len() > 16 {
        return false;
    }

    // Try to parse the data as hex
    can_data.chars().all(|c| c.is_ascii_hexdigit())
}
