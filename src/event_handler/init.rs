use crate::slint_generatedAppWindow::{socket_info, AppWindow};
#[cfg(target_os = "windows")]
use pcan_basic::hw::attached_channels as available_interfaces;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel, Weak};
#[cfg(target_os = "linux")]
use socketcan::available_interfaces;
use std::{process::exit, time::Duration};
pub struct Init<'a> {
    pub ui_handle: &'a Weak<AppWindow>,
}

impl<'a> Init<'a> {
    pub fn run(&self) {
        let mut previous_interfaces = Vec::default();
        loop {
            match available_interfaces() {
                Ok(interface) => {
                    if interface.is_empty() {
                        let _ = self.ui_handle.upgrade_in_event_loop(move |ui| {
                            let socket_info = socket_info {
                                index: ModelRc::new(VecModel::from(Vec::default())),
                                name: ModelRc::new(VecModel::from(Vec::default())),
                            };
                            ui.set_can_sockets(socket_info);
                            ui.set_init_string(SharedString::from("No CAN device found !"));
                        });
                    } else {
                        #[cfg(target_os = "linux")]
                        if previous_interfaces != interface {
                            let interface_clone = interface.clone();
                            previous_interfaces = interface.clone();
                            let _ = self.ui_handle.upgrade_in_event_loop(move |ui| {
                                ui.set_init_string(SharedString::from(format!(
                                    "Found {} CAN devices\n Please select your device ",
                                    interface.len()
                                )));
                            });

                            let _ = self.ui_handle.upgrade_in_event_loop(move |ui| {
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
                        #[cfg(target_os = "windows")]
                        {
                            let mut interface_names = Vec::default();
                            let mut interface_index = Vec::default();
                            let mut count = 0;
                            for channel in interface {
                                let interface_name = SharedString::from(format!(
                                    "{}(0x{:02X})",
                                    channel.device_name(),
                                    channel.channel_information.device_id
                                ));
                                interface_names.push(interface_name);
                                interface_index
                                    .push(channel.channel_information.channel_handle as i32);
                                count += 1;
                            }
                            if previous_interfaces != interface_names {
                                previous_interfaces = interface_names.clone();
                                let _ = self.ui_handle.upgrade_in_event_loop(move |ui| {
                                    ui.set_init_string(SharedString::from(format!(
                                        "Found {} CAN devices\n Please select your device ",
                                        count
                                    )));
                                    let socket_info = socket_info {
                                        index: ModelRc::new(VecModel::from(interface_index)),
                                        name: ModelRc::new(VecModel::from(interface_names)),
                                    };
                                    ui.set_can_sockets(socket_info);
                                });
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = self.ui_handle.upgrade_in_event_loop(move |ui| {
                        ui.set_init_string(SharedString::from(format!(
                            "Can't get device list: {:?}",
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
            let _ = self.ui_handle.upgrade_in_event_loop(move |ui| {
                if !ui.window().is_visible() {
                    exit(1);
                }
            });
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}
