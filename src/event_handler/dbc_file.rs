use can_dbc::DBC;
use rfd::FileDialog;
use slint::{Model, VecModel};
use slint::{ModelRc, SharedString, Weak};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::Sender;

use crate::slint_generatedAppWindow::AppWindow;
use crate::slint_generatedAppWindow::CanData;
use crate::slint_generatedAppWindow::CanSignal;

use super::{EVEN_COLOR, ODD_COLOR};
pub struct DBCFile<'a> {
    pub ui_handle: &'a Weak<AppWindow>,
    pub mspc_tx: &'a Sender<DBC>,
}

impl<'a> DBCFile<'a> {
    pub fn process_dbc_file(&self) {
        let dbc_data = Self::read_dbc_data(Self::open_dbc_file());
        match dbc_data {
            Some(dbc) => {
                let dbc_clone = dbc.clone();
                let ui = self.ui_handle.unwrap();
                ui.set_is_new_dbc(true);
                ui.set_is_filter(false);
                // Remove all filter data when open new DBC file
                let list_filter: Vec<CanData> = [].to_vec();
                ui.set_filter_messages(Rc::new(VecModel::from(list_filter.clone())).into());

                let message_vec: Rc<VecModel<CanData>> = Rc::new(VecModel::from(
                    [CanData {
                        can_id: SharedString::from("default"),
                        packet_name: SharedString::from("default"),
                        signal_value: ModelRc::default(),
                        counter: 0,
                        raw_can: SharedString::from("default"),
                        color: ODD_COLOR,
                        circle_time: "0.0".into(),
                        time_stamp: "0".into(),
                    }]
                    .to_vec(),
                ));

                let filter_list: Rc<VecModel<SharedString>> =
                    Rc::new(VecModel::from([SharedString::from("default")].to_vec()));
                for (message_count, message) in dbc.messages().iter().enumerate() {
                    let can_signals: Rc<VecModel<CanSignal>> = Rc::new(VecModel::from(
                        [CanSignal {
                            signal_name: SharedString::from("default"),
                            signal_value: SharedString::from("default"),
                            factor: SharedString::from("default"),
                            unit: SharedString::from("default"),
                        }]
                        .to_vec(),
                    ));
                    for (signal_count, signal) in message.signals().iter().enumerate() {
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
                    }

                    let can_data = CanData {
                        can_id: SharedString::from(format!(
                            "{:08X}",
                            message.message_id().raw() & !0x80000000
                        )),
                        packet_name: SharedString::from(message.message_name()),
                        signal_value: can_signals.into(),
                        counter: 0,
                        raw_can: SharedString::from(""),
                        color: if message_count % 2 == 0 {
                            EVEN_COLOR
                        } else {
                            ODD_COLOR
                        },
                        circle_time: "0.0".into(),
                        time_stamp: "0".into(),
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
                }

                ui.set_messages(message_vec.into());
                let _ = self.mspc_tx.send(dbc_clone);
            }
            None => {
                println!("ERR: Failed to read DBC data");
            }
        }
    }
    fn open_dbc_file() -> Option<PathBuf> {
        FileDialog::new()
            .add_filter("dbc", &["dbc"])
            .set_directory("./")
            .pick_file()
    }

    fn read_dbc_data(file: Option<PathBuf>) -> Option<DBC> {
        if let Some(path_dbc) = file {
            if path_dbc.is_file() {
                let mut f = File::open(path_dbc.as_path()).unwrap();
                let mut buffer = Vec::new();
                f.read_to_end(&mut buffer).unwrap();

                Some(can_dbc::DBC::from_slice(&buffer).expect("Failed to parse dbc file"))
            } else {
                None
            }
        } else {
            None
        }
    }
}
