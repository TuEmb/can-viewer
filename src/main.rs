use event_handler::can_handler::CanHandler;
use rfd::FileDialog;
use slint::{Model, ModelRc, SharedString, VecModel};
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::rc::Rc;
use std::sync::mpsc;

mod event_handler;
use event_handler::PacketFilter;

slint::include_modules!();

fn main() -> io::Result<()> {
    let ui = AppWindow::new().unwrap();
    let (tx, rx) = mpsc::channel();

    // Handle open file event
    ui.on_open_dbc_file({
        let ui_handle = ui.as_weak();
        move || {
            let tx_clone = tx.clone();
            let ui_update = ui_handle.clone();
            std::thread::spawn(move || {
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
                            ui.set_is_new_dbc(true);
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
                        let _ = tx_clone.send(clone_dbc);
                    }
                }
            });
        }
    });

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

    let ui_handle = ui.as_weak();
    ui.on_filter_id(move |filter, is_check| {
        let packet_filter = PacketFilter {
            ui_handle: &ui_handle,
            filter,
            is_check,
        };
        packet_filter.process_filter();
    });

    let _ = ui.run().unwrap();
    Ok(())
}
