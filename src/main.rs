use std::io;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

mod event_handler;
use can_dbc::DBC;
use event_handler::{CanHandler, DBCFile, DebugHandler, Init, PacketFilter};
#[cfg(target_os = "windows")]
use pcan_basic::{bus::UsbBus, socket::usb::UsbCanSocket};
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
            use event_handler::p_can_bitrate;
            let ui = ui_handle.unwrap();
            let get_device_handle = match ui.get_can_sockets().index.row_data(_index as usize) {
                Some(device) => device,
                None => {
                    ui.set_init_string(SharedString::from("No device found!!!"));
                    return;
                }
            };
            let usb_can = UsbBus::try_from(get_device_handle as u16).unwrap();
            let ui_handle = ui.as_weak();
            let baudrate = p_can_bitrate(&bitrate).unwrap();
            match UsbCanSocket::open(usb_can, baudrate) {
                Ok(socket) => {
                    ui_handle.unwrap().set_is_init(true);
                    let _ = start_tx_1.send((socket, bitrate));
                }
                Err(e) => {
                    ui_handle
                        .unwrap()
                        .set_init_string(SharedString::from(format!("Failed to start: {:?}", e)));
                }
            }
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

    ui.run().unwrap();
    Ok(())
}
