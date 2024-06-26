use std::io;
use std::sync::mpsc;

mod event_handler;
use dialoguer::{theme::ColorfulTheme, Select};
use event_handler::{CanHandler, DBCFile, PacketFilter};
use socketcan::available_interfaces;

slint::include_modules!();

fn main() -> io::Result<()> {
    // Find available socket CAN
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
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose an socket CAN interface:")
        .items(&socket_if[..])
        .default(0)
        .interact()
        .unwrap();

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
        let mut can_handler = CanHandler {
            iface: &socket_if[selection],
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
