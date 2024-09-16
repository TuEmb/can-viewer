use std::io;
use std::process::exit;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

mod event_handler;
use can_dbc::DBC;
use event_handler::{CanHandler, DBCFile, PacketFilter};
#[cfg(target_os = "windows")]
use pcan_basic::{
    bus::UsbBus,
    hw::attached_channels,
    socket::{usb::UsbCanSocket, Baudrate},
};
#[cfg(target_os = "linux")]
use privilege_rs::privilege_request;
#[cfg(target_os = "windows")]
use slint::Model;
use slint::{ModelRc, SharedString, VecModel};
#[cfg(target_os = "linux")]
use socketcan::available_interfaces;
#[cfg(target_os = "windows")]
use winapi::um::wincon::FreeConsole;

slint::include_modules!();

#[tokio::main]
async fn main() -> io::Result<()> {
    privilege_request();
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
    #[cfg(target_os = "linux")]
    tokio::spawn(async move {
        let mut previous_interfaces = Vec::default();
        loop {
            match available_interfaces() {
                Ok(interface) => {
                    if interface.is_empty() {
                        let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                            let socket_info = socket_info {
                                index: ModelRc::new(VecModel::from(Vec::default())),
                                name: ModelRc::new(VecModel::from(Vec::default())),
                            };
                            ui.set_can_sockets(socket_info);
                            ui.set_init_string(SharedString::from("No CAN device found !"));
                        });
                    } else if previous_interfaces != interface {
                        let interface_clone = interface.clone();
                        previous_interfaces = interface.clone();
                        let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                            ui.set_init_string(SharedString::from(format!(
                                "Found {} CAN devices\n Please select your device ",
                                interface.len()
                            )));
                        });

                        let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                            let convert_shared_string: Vec<SharedString> = interface_clone
                                .into_iter()
                                .map(SharedString::from)
                                .collect();
                            let socket_info = socket_info {
                                index: ModelRc::new(VecModel::from(Vec::default())),
                                name: ModelRc::new(VecModel::from(convert_shared_string)),
                            };
                            ui.set_can_sockets(socket_info);
                        });
                    }
                }
                Err(e) => {
                    let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                        ui.set_init_string(SharedString::from(format!(
                            "Can't get device list: {}",
                            e
                        )));
                        let socket_info = socket_info {
                            index: ModelRc::new(VecModel::from(Vec::default())),
                            name: ModelRc::new(VecModel::from(Vec::default())),
                        };
                        ui.set_can_sockets(socket_info);
                    });
                }
            };
            let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                if !ui.window().is_visible() {
                    exit(1);
                }
            });
            tokio::time::sleep(Duration::from_micros(50)).await;
        }
    });

    #[cfg(target_os = "windows")]
    tokio::spawn(async move {
        let mut previous_sockets = Vec::default();
        loop {
            // get channel_handle
            match attached_channels() {
                Ok(channels) => {
                    if channels.is_empty() {
                        let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                            ui.set_init_string(SharedString::from("No CAN device found !"));
                        });
                    } else {
                        let mut can_socket_names = Vec::default();
                        let mut can_socket_index = Vec::default();
                        let mut count = 0;
                        for channel in channels {
                            let socket_name = SharedString::from(format!(
                                "{}(0x{:02X})",
                                channel.device_name(),
                                channel.channel_information.device_id
                            ));
                            can_socket_names.push(socket_name);
                            can_socket_index
                                .push(channel.channel_information.channel_handle as i32);
                            count += 1;
                        }
                        if previous_sockets != can_socket_names {
                            previous_sockets = can_socket_names.clone();
                            let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                                ui.set_init_string(SharedString::from(format!(
                                    "Found {} CAN devices\n Please select your device ",
                                    count
                                )));
                                let socket_info = socket_info {
                                    index: ModelRc::new(VecModel::from(can_socket_index)),
                                    name: ModelRc::new(VecModel::from(can_socket_names)),
                                };
                                ui.set_can_sockets(socket_info);
                            });
                        }
                    }
                }
                Err(e) => {
                    let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                        ui.set_init_string(SharedString::from(format!(
                            "Can't get device list: {:?}",
                            e
                        )));
                    });
                }
            }
            let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                if !ui.window().is_visible() {
                    exit(1);
                }
            });
            tokio::time::sleep(Duration::from_micros(50)).await;
        }
    });

    let (start_tx, start_rx) = mpsc::channel();
    // Handle start event
    let ui_handle = ui.as_weak();
    ui.on_start(move |_name, _index, bitrate| {
        // start_tx.send((_name, _index));
        #[cfg(target_os = "linux")]
        {
            let ui = ui_handle.unwrap();
            if _name.is_empty() {
                ui.set_init_string(SharedString::from("No device found!!!"));
            } else {
                ui.set_is_init(true);
                let _ = start_tx.send((_name, bitrate));
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
            let ui_handle = ui.as_weak();
            match UsbCanSocket::open(usb_can, Baudrate::Baud250K) {
                Ok(socket) => {
                    ui_handle.unwrap().set_is_init(true);
                    let _ = start_tx.send(socket);
                }
                Err(e) => {
                    ui_handle
                        .unwrap()
                        .set_init_string(SharedString::from(format!("Failed to start: {:?}", e)));
                }
            }
        }
    });

    let ui_handle = ui.as_weak();
    tokio::spawn(async move {
        if let Ok((can_if, bitrate)) = start_rx.recv() {
            let mut can_handler = CanHandler {
                #[cfg(target_os = "windows")]
                iface: can_if,
                #[cfg(target_os = "linux")]
                iface: &can_if,
                ui_handle: &ui_handle,
                mspc_rx: &rx,
                bitrate: bitrate.to_string(),
            };
            loop {
                can_handler.process_can_messages();
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
    ui.run().unwrap();
    Ok(())
}
