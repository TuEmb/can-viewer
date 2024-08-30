use std::io;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

mod event_handler;
use can_dbc::DBC;
use event_handler::{CanHandler, DBCFile, PacketFilter};
#[cfg(target_os = "windows")]
use pcan_basic::{
    bus::UsbBus,
    hw::attached_channels,
    socket::{usb::UsbCanSocket, Baudrate},
};
#[cfg(target_os = "windows")]
use slint::Model;
use slint::{ModelRc, SharedString, VecModel};
#[cfg(target_os = "linux")]
use socketcan::available_interfaces;
#[cfg(target_os = "windows")]
use winapi::um::wincon::FreeConsole;

slint::include_modules!();

fn main() -> io::Result<()> {
    let ui = AppWindow::new().unwrap();
    let (tx, rx) = mpsc::channel::<DBC>();
    // Wrap `rx` in an Arc<Mutex<>> so it can be shared safely across threads
    let rx = Arc::new(Mutex::new(rx));
    #[cfg(target_os = "windows")]
    unsafe {
        FreeConsole(); // This detaches the console from the application
    }

    // Find available socket CAN
    #[cfg(target_os = "linux")]
    let ui_handle = ui.as_weak();
    #[cfg(target_os = "linux")]
    std::thread::spawn(move || loop {
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
                } else {
                    let interface_clone = interface.clone();
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
                    ui.set_init_string(SharedString::from(format!("Can't get device list: {}", e)));
                    let socket_info = socket_info {
                        index: ModelRc::new(VecModel::from(Vec::default())),
                        name: ModelRc::new(VecModel::from(Vec::default())),
                    };
                    ui.set_can_sockets(socket_info);
                });
            }
        };
    });

    #[cfg(target_os = "windows")]
    {
        // get channel_handle
        let mut can_socket_names = Vec::default();
        let mut can_socket_index = Vec::default();
        match attached_channels() {
            Ok(channels) => {
                if channels.is_empty() {
                    ui.set_init_string(SharedString::from("No CAN device found !"));
                } else {
                    ui.set_init_string(SharedString::from(format!(
                        "Found {} CAN devices\n Please select your device ",
                        channels.len()
                    )));
                    for channel in channels {
                        let socket_name = SharedString::from(format!(
                            "{}(0x{:02X})",
                            channel.device_name(),
                            channel.channel_information.device_id
                        ));
                        can_socket_names.push(socket_name);
                        can_socket_index.push(channel.channel_information.channel_handle as i32);
                    }
                    let socket_info = socket_info {
                        index: ModelRc::new(VecModel::from(can_socket_index)),
                        name: ModelRc::new(VecModel::from(can_socket_names)),
                    };
                    ui.set_can_sockets(socket_info);
                }
            }
            Err(e) => {
                ui.set_init_string(SharedString::from(format!(
                    "Can't get device list: {:?}",
                    e
                )));
            }
        }
    }

    // Handle start event
    let ui_handle = ui.as_weak();
    ui.on_start(move |_name, _index| {
        #[cfg(target_os = "linux")]
        {
            let ui = ui_handle.unwrap();
            if _name.is_empty() {
                ui.set_init_string(SharedString::from("No device found!!!"));
            } else {
                ui.set_is_init(true);
                let ui_handle = ui.as_weak();
                let rx = Arc::clone(&rx);
                std::thread::spawn(move || {
                    let mut can_handler = CanHandler {
                        #[cfg(target_os = "linux")]
                        iface: &_name,
                        ui_handle: &ui_handle,
                        mspc_rx: &rx,
                    };
                    loop {
                        can_handler.process_can_messages();
                    }
                });
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
                    let rx = Arc::clone(&rx);
                    std::thread::spawn(move || {
                        let mut can_handler = CanHandler {
                            #[cfg(target_os = "linux")]
                            iface: &socket_if[selection],
                            #[cfg(target_os = "windows")]
                            iface: socket,
                            ui_handle: &ui_handle,
                            mspc_rx: &rx,
                        };
                        loop {
                            can_handler.process_can_messages();
                        }
                    });
                }
                Err(e) => {
                    ui_handle
                        .unwrap()
                        .set_init_string(SharedString::from(format!("Failed to start: {:?}", e)));
                }
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
