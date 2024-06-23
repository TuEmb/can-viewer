use can_dbc::DBC;
use slint::{Model, VecModel, Weak};
use slint::{ModelRc, SharedString};
use socketcan::{CanSocket, EmbeddedFrame, Frame, Socket};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc::Receiver;
use std::thread::sleep;
use std::time::Duration;

use crate::slint_generatedAppWindow::AppWindow;
use crate::slint_generatedAppWindow::CanData;
use crate::slint_generatedAppWindow::CanSignal;
pub struct CanHandler<'a> {
    pub iface: &'a str,
    pub ui_handle: &'a Weak<AppWindow>,
    pub mspc_rx: &'a Receiver<DBC>,
}

impl<'a> CanHandler<'a> {
    pub fn process_can_messages(&mut self) {
        if let Ok(dbc) = self.mspc_rx.try_recv() {
            let can_socket = self.open_can_socket();
            self.process_ui_events(dbc, can_socket);
        }
    }

    fn open_can_socket(&self) -> CanSocket {
        loop {
            match CanSocket::open(self.iface) {
                Ok(socket) => break socket,
                Err(e) => {
                    println!(
                        "ERR: Failed to open socket {} - {}\nTry to re-connect...",
                        self.iface, e
                    );
                    sleep(Duration::from_millis(1000));
                }
            }
        }
    }

    fn process_ui_events(&self, dbc: DBC, can_socket: CanSocket) {
        let _ = self.ui_handle.upgrade_in_event_loop(move |ui| loop {
            if ui.get_is_new_dbc() {
                ui.set_is_new_dbc(false);
                break;
            }
            if let Ok(frame) = can_socket.read_frame() {
                let frame_id = frame.raw_id() & !0x80000000;
                for message in dbc.messages() {
                    if frame_id == (message.message_id().raw() & !0x80000000) {
                        let padding_data = Self::pad_to_8_bytes(frame.data());
                        let signal_data = message.parse_from_can(&padding_data);
                        let is_filter = ui.get_is_filter();
                        let messages: ModelRc<CanData> = if !is_filter {
                            ui.get_messages()
                        } else {
                            ui.get_filter_messages()
                        };
                        Self::update_ui_with_signals(&messages, frame_id, signal_data);
                    }
                }
            }
        });
    }

    fn update_ui_with_signals(
        messages: &ModelRc<CanData>,
        frame_id: u32,
        signal_data: HashMap<String, f32>,
    ) {
        for (message_count, message) in messages.iter().enumerate() {
            if message.can_id == frame_id.to_string() {
                let can_signals = Self::create_can_signals(&message, &signal_data);
                messages.set_row_data(
                    message_count,
                    CanData {
                        can_id: message.can_id.clone(),
                        packet_name: message.packet_name.clone(),
                        signal_value: can_signals.into(),
                        counter: message.counter + 1,
                    },
                );
                break;
            }
        }
    }

    fn create_can_signals(
        message: &CanData,
        signal_data: &HashMap<String, f32>,
    ) -> Rc<VecModel<CanSignal>> {
        let can_signals = Rc::new(VecModel::from(
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
            if let Some((key, value)) = signal_data.get_key_value(signal.signal_name.as_str()) {
                let can_signal = CanSignal {
                    signal_name: SharedString::from(key),
                    signal_value: SharedString::from(format!("{}", value)),
                    factor: signal.factor.clone(),
                    unit: signal.unit.clone(),
                };
                if signal_count == 0 {
                    can_signals.set_row_data(signal_count, can_signal);
                } else {
                    can_signals.push(can_signal);
                }
                signal_count += 1;
            }
        }
        can_signals
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
}
