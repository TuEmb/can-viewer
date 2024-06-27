use std::io;
use std::sync::mpsc;

mod event_handler;
#[cfg(target_os = "linux")]
use dialoguer::{theme::ColorfulTheme, Select};
#[cfg(target_os = "windows")]
use event_handler::can_handler::PcanDriver;
use event_handler::{CanHandler, DBCFile, PacketFilter};
#[cfg(target_os = "linux")]
use socketcan::available_interfaces;

slint::include_modules!();

fn main() -> io::Result<()> {
    // Find available socket CAN
    #[cfg(target_os = "linux")]
    let socket_if = match available_interfaces() {
        Ok(interface) => {
            if interface.is_empty() {
                println!("ERR: Can't find any socket can interfaces: length is 0");
                return Ok(());
            } else {
                interface
            }
        }
        Err(e) => {
            println!("ERR: Can't find any socket can interfaces: {}", e);
            return Ok(());
        }
    };

    // Create and selectable list for socket CAN
    #[cfg(target_os = "linux")]
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose an socket CAN interface:")
        .items(&socket_if[..])
        .default(0)
        .interact()
        .unwrap();
    #[cfg(target_os = "windows")]
    {
        let can_interface = pcan_basic::Interface::init(0x011C);
        match can_interface {
            Ok(can) => {
                drop(can);
            }
            Err(e) => {
                println!("ERROR: Can't find any PCAN interface input: {}", e);
                return Ok(());
            }
        }
    }

    let ui = AppWindow::new().unwrap();
    let (tx, rx) = mpsc::channel();

    // Handle open file event
    let ui_handle = ui.as_weak();
    ui.on_open_dbc_file(move || {
        let dbc_handle = DBCFile {
            ui_handle: &ui_handle,
            mspc_tx: &tx.clone(),
        };

        dbc_handle.process_dbc_file();
    });

    // Create thread to handle CAN packet comming and
    // update to UI the signal - value
    let ui_handle = ui.as_weak();
    std::thread::spawn(move || {
        #[cfg(target_os = "windows")]
        let can_interface = loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            match pcan_basic::Interface::init(0x011C) {
                Ok(can_if) => break PcanDriver(can_if),
                Err(e) => {
                    println!("ERR: Can't find any PCAN Interface: {}", e);
                    println!("Trying to reconnect ...");
                    continue;
                }
            }
        };

        let mut can_handler = CanHandler {
            #[cfg(target_os = "linux")]
            iface: &socket_if[selection],
            #[cfg(target_os = "windows")]
            iface: can_interface,
            ui_handle: &ui_handle,
            mspc_rx: &rx,
        };
        loop {
            can_handler.process_can_messages();
        }
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

    ui.run().unwrap();
    Ok(())
}
