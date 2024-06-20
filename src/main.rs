use rfd::FileDialog;
use slint::{Model, ModelRc, SharedString, VecModel};
use socketcan::{CanSocket, EmbeddedFrame, Frame, Socket};
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::rc::Rc;
use tokio::sync::mpsc;

slint::include_modules!();

#[tokio::main]
async fn main() -> io::Result<()> {
    let ui = AppWindow::new().unwrap();
    let (tx, mut rx) = mpsc::channel::<can_dbc::DBC>(1);

    ui.on_open_dbc_file({
        let ui_handle = ui.as_weak();
        move || {
            let tx_clone = tx.clone();
            // let ui_handle = ui.as_weak();
            let ui_update = ui_handle.clone();
            tokio::spawn(async move {
                let files = FileDialog::new()
                    .add_filter("dbc", &["dbc"])
                    .set_directory("./")
                    .pick_file();
                if let Some(path_dbc) = files {
                    if path_dbc.is_file() {
                        let mut f = File::open(path_dbc.as_path()).unwrap();
                        let mut buffer = Vec::new();
                        f.read_to_end(&mut buffer).unwrap();

                        let dbc =
                            can_dbc::DBC::from_slice(&buffer).expect("Failed to parse dbc file");
                        let clone_dbc = dbc.clone();
                        let _ = ui_update.upgrade_in_event_loop(move |ui| {
                            let message_vec: Rc<VecModel<CanData>> = Rc::new(VecModel::from(
                                [CanData {
                                    can_id: SharedString::from("default"),
                                    packet_name: SharedString::from("default"),
                                    signal_value: ModelRc::default(),
                                    counter: 0,
                                }]
                                .to_vec(),
                            ));

                            let filter_list: Rc<VecModel<SharedString>> =
                                Rc::new(VecModel::from([SharedString::from("default")].to_vec()));
                            let mut message_count = 0;
                            for message in dbc.messages() {
                                let can_signals: Rc<VecModel<CanSignal>> = Rc::new(VecModel::from(
                                    [CanSignal {
                                        signal_name: SharedString::from("default"),
                                        signal_value: SharedString::from("default"),
                                        factor: SharedString::from("default"),
                                        unit: SharedString::from("default"),
                                    }]
                                    .to_vec(),
                                ));
                                let mut signal_count = 0;
                                for signal in message.signals() {
                                    let can_signal = CanSignal {
                                        signal_name: SharedString::from(signal.name()),
                                        signal_value: SharedString::from("0"),
                                        factor: SharedString::from(signal.factor.to_string()),
                                        unit: SharedString::from(signal.unit()),
                                    };
                                    if signal_count == 0 {
                                        can_signals.set_row_data(signal_count, can_signal)
                                    } else {
                                        can_signals.push(can_signal);
                                    }
                                    signal_count += 1;
                                }

                                let can_data = CanData {
                                    can_id: SharedString::from(format!(
                                        "{:08X}",
                                        message.message_id().raw() & !0x80000000
                                    )),
                                    packet_name: SharedString::from(message.message_name()),
                                    signal_value: can_signals.into(),
                                    counter: 0,
                                };

                                if message_count == 0 {
                                    filter_list.set_row_data(
                                        message_count,
                                        SharedString::from(format!(
                                            "{:08X}",
                                            message.message_id().raw() & !0x80000000
                                        )),
                                    );
                                    message_vec.set_row_data(message_count, can_data)
                                } else {
                                    message_vec.push(can_data);
                                    filter_list.push(SharedString::from(format!(
                                        "{:08X}",
                                        message.message_id().raw() & !0x80000000
                                    )));
                                }
                                message_count += 1;
                            }

                            ui.set_messages(message_vec.into());
                        });
                        let _ = tx_clone.send(clone_dbc).await;
                    }
                }
            });
        }
    });

    let ui_handle = ui.as_weak();
    tokio::spawn(async move {
        while let Some(dbc) = rx.recv().await {
            let can_socket = CanSocket::open("can0").unwrap();
            while let Ok(frame) = can_socket.read_frame() {
                let frame_id = frame.raw_id() & !0x80000000;
                for message in dbc.messages() {
                    if frame_id == (message.message_id().raw() & !0x80000000) {
                        let padding_data = pad_to_8_bytes(frame.data());
                        let signal_data = message.parse_from_can(&padding_data);
                        let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                            let is_filter = ui.get_is_filter();
                            let messages: ModelRc<CanData> = if !is_filter {
                                ui.get_messages()
                            } else {
                                ui.get_filter_messages()
                            };
                            let mut message_count = 0;
                            for message in messages.clone().iter() {
                                if message.can_id == SharedString::from(format!("{:08X}", frame_id))
                                {
                                    let can_signals: Rc<VecModel<CanSignal>> =
                                        Rc::new(VecModel::from(
                                            [CanSignal {
                                                signal_name: SharedString::from("default"),
                                                signal_value: SharedString::from("default"),
                                                factor: SharedString::from("default"),
                                                unit: SharedString::from("default"),
                                            }]
                                            .to_vec(),
                                        ));
                                    let mut signal_count = 0;
                                    for signal in message.signal_value.iter() {
                                        if let Some((key, value)) =
                                            signal_data.get_key_value(signal.signal_name.as_str())
                                        {
                                            let can_signal = CanSignal {
                                                signal_name: SharedString::from(key),
                                                signal_value: SharedString::from(format!(
                                                    "{}",
                                                    value
                                                )),
                                                factor: signal.factor,
                                                unit: signal.unit,
                                            };
                                            if signal_count == 0 {
                                                can_signals.set_row_data(signal_count, can_signal)
                                            } else {
                                                can_signals.push(can_signal);
                                            }
                                            signal_count += 1;
                                        }
                                    }
                                    messages.set_row_data(
                                        message_count,
                                        CanData {
                                            can_id: message.can_id.clone(),
                                            packet_name: message.packet_name.clone(),
                                            signal_value: can_signals.clone().into(),
                                            counter: message.counter + 1,
                                        },
                                    );
                                    break;
                                }
                                message_count += 1;
                            }
                        });
                    }
                }
            }
        }
    });
    let ui_handle = ui.as_weak();
    ui.on_filter_id(move |filter, checked| {
        let ui = ui_handle.unwrap();
        let mut list_filter: Vec<CanData> = ui.get_filter_messages().iter().collect();
        if checked {
            // Add filter ID
            list_filter.push(filter);
        } else {
            // Remove filter ID
            let mut filter_count = 0;
            for can_filter in list_filter.clone() {
                if can_filter.can_id == filter.can_id {
                    list_filter.remove(filter_count);
                }
                filter_count += 1;
            }
        }

        ui.set_filter_messages(Rc::new(VecModel::from(list_filter.clone())).into());

        if list_filter.is_empty() {
            ui.set_is_filter(false);
        } else {
            ui.set_is_filter(true);
        }
    });

    let _ = ui.run();
    Ok(())
}

fn pad_to_8_bytes(data: &[u8]) -> Vec<u8> {
    // Convert the byte slice to a Vec<u8>
    let mut padded_data = data.to_vec();

    // Calculate the number of padding bytes needed
    let padding_needed = 8usize.saturating_sub(padded_data.len());

    // Extend the vector with zeros (or another byte) to make it 8 bytes long
    padded_data.extend(std::iter::repeat(0).take(padding_needed));

    // Return the padded vector
    padded_data
}
