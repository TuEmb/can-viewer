use std::io;
use std::sync::mpsc;

mod event_handler;
use event_handler::{CanHandler, DBCFile, PacketFilter};

slint::include_modules!();

fn main() -> io::Result<()> {
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
            iface: "can0",
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
