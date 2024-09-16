use can_dbc::DBC;
use chrono::Utc;
#[cfg(target_os = "windows")]
use pcan_basic::socket::usb::UsbCanSocket;
use slint::{Model, VecModel, Weak};
use slint::{ModelRc, SharedString};
#[cfg(target_os = "linux")]
use socketcan::{CanInterface, CanSocket, EmbeddedFrame, Frame, Socket};
use std::collections::HashMap;
use std::fmt::Write;
use std::rc::Rc;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;
use std::time::Instant;

use crate::slint_generatedAppWindow::AppWindow;
use crate::slint_generatedAppWindow::CanData;
use crate::slint_generatedAppWindow::CanSignal;
pub struct CanHandler<'a> {
    #[cfg(target_os = "linux")]
    pub iface: &'a str,
    #[cfg(target_os = "windows")]
    pub iface: UsbCanSocket,
    pub ui_handle: &'a Weak<AppWindow>,
    pub mspc_rx: &'a Arc<Mutex<Receiver<DBC>>>,
    pub bitrate: String,
}

static mut NEW_DBC_CHECK: bool = false;
use super::{EVEN_COLOR, ODD_COLOR};

impl<'a> CanHandler<'a> {
    pub fn process_can_messages(&mut self) {
        if let Ok(dbc) = self.mspc_rx.lock().unwrap().try_recv() {
            #[cfg(target_os = "linux")]
            {
                let can_if = CanInterface::open(self.iface).unwrap();
                let _ = can_if.bring_down();
                let _ = can_if.set_bitrate(self.bitrate().unwrap(), None);
                let _ = can_if.bring_up();
                let can_socket = self.open_can_socket();
                self.process_ui_events(dbc, can_socket, can_if);
            }
            #[cfg(target_os = "windows")]
            self.process_ui_events(dbc);
        }
        sleep(Duration::from_millis(10));
    }
    #[cfg(target_os = "linux")]
    fn open_can_socket(&self) -> CanSocket {
        loop {
            match CanSocket::open(self.iface) {
                Ok(socket) => {
                    let _ = socket.set_nonblocking(true);
                    break socket;
                }
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
    #[cfg(target_os = "linux")]
    fn process_ui_events(&self, dbc: DBC, can_socket: CanSocket, can_if: CanInterface) {
        let mut start_bus_load = Instant::now();
        let mut total_bits = 0;
        loop {
            let bus_state = match can_if.state().unwrap().unwrap() {
                socketcan::nl::CanState::ErrorActive => "ERR_ACTIVE",
                socketcan::nl::CanState::ErrorWarning => "ERR_WARNING",
                socketcan::nl::CanState::ErrorPassive => "ERR_PASSIVE",
                socketcan::nl::CanState::BusOff => "BUSOFF",
                socketcan::nl::CanState::Stopped => "STOPPED",
                socketcan::nl::CanState::Sleeping => "SLEEPING",
            };
            let bitrate = can_if.bit_rate().unwrap().unwrap();
            let busload = if start_bus_load.elapsed() >= Duration::from_millis(1000) {
                start_bus_load = Instant::now();
                let bus_load = (total_bits as f64 / bitrate as f64) * 100.0;
                total_bits = 0;
                bus_load
            } else {
                0.0
            };
            let _ = self.ui_handle.upgrade_in_event_loop(move |ui| unsafe {
                if ui.get_is_new_dbc() {
                    if ui.get_is_first_open() {
                        ui.set_is_first_open(false);
                    } else {
                        NEW_DBC_CHECK = true;
                    }
                    ui.set_is_new_dbc(false);
                }
                ui.set_state(bus_state.into());
                ui.set_bitrate(bitrate as i32);
                if busload > 0.0 {
                    ui.set_bus_load(busload as i32);
                }
            });
            unsafe {
                if NEW_DBC_CHECK {
                    NEW_DBC_CHECK = false;
                    break;
                }
            }
            if let Ok(frame) = can_socket.read_frame() {
                total_bits += (frame.len() + 6) * 8; // Data length + overhead (approximation)
                let frame_id = frame.raw_id() & !0x80000000;
                for message in dbc.messages() {
                    if frame_id == (message.message_id().raw() & !0x80000000) {
                        let padding_data = Self::pad_to_8_bytes(frame.data());
                        let hex_string = Self::array_to_hex_string(frame.data());
                        let signal_data = message.parse_from_can(&padding_data);
                        let _ = self.ui_handle.upgrade_in_event_loop(move |ui| {
                            let is_filter = ui.get_is_filter();
                            let messages: ModelRc<CanData> = if !is_filter {
                                ui.get_messages()
                            } else {
                                ui.get_filter_messages()
                            };
                            Self::update_ui_with_signals(
                                &messages,
                                frame_id,
                                signal_data,
                                hex_string,
                            );
                        });
                    }
                }
            }
        }
    }
    #[cfg(target_os = "windows")]
    fn process_ui_events(&mut self, dbc: DBC) {
        use pcan_basic::{error::PcanError, socket::RecvCan};
        let mut start_bus_load = Instant::now();
        let mut total_bits = 0;
        loop {
            let bitrate = self.bitrate().unwrap();
            let busload = if start_bus_load.elapsed() >= Duration::from_millis(1000) {
                start_bus_load = Instant::now();
                let bus_load = (total_bits as f64 / bitrate as f64) * 100.0;
                total_bits = 0;
                bus_load
            } else {
                0.0
            };
            let _ = self.ui_handle.upgrade_in_event_loop(move |ui| unsafe {
                if ui.get_is_new_dbc() {
                    if ui.get_is_first_open() {
                        ui.set_is_first_open(false);
                    } else {
                        NEW_DBC_CHECK = true;
                    }
                    ui.set_is_new_dbc(false);
                }
                ui.set_bitrate(bitrate as i32);
                if busload > 0.0 {
                    ui.set_bus_load(busload as i32);
                }
            });
            unsafe {
                if NEW_DBC_CHECK {
                    NEW_DBC_CHECK = false;
                    break;
                }
            }
            match self.iface.recv_frame() {
                Ok(frame) => {
                    let _ = self.ui_handle.upgrade_in_event_loop(move |ui| {
                        ui.set_state("OK".into());
                    });
                    total_bits += (frame.dlc() as u32 + 6) * 8; // Data length + overhead (approximation)
                    let id = frame.can_id();
                    let frame_id = id & !0x80000000;
                    for message in dbc.messages() {
                        if frame_id == (message.message_id().raw() & !0x80000000) {
                            let padding_data = Self::pad_to_8_bytes(frame.data());
                            let hex_string = Self::array_to_hex_string(frame.data());
                            let signal_data = message.parse_from_can(&padding_data);
                            let _ = self.ui_handle.upgrade_in_event_loop(move |ui| {
                                let is_filter = ui.get_is_filter();
                                let messages: ModelRc<CanData> = if !is_filter {
                                    ui.get_messages()
                                } else {
                                    ui.get_filter_messages()
                                };
                                Self::update_ui_with_signals(
                                    &messages,
                                    frame_id,
                                    signal_data,
                                    hex_string,
                                );
                            });
                        }
                    }
                }
                Err(e) => {
                    let _ = self.ui_handle.upgrade_in_event_loop(move |ui| {
                        if e != PcanError::QrcvEmpty {
                            ui.set_state(format!("{:?}", e).into());
                        }
                    });
                }
            }
        }
    }

    fn update_ui_with_signals(
        messages: &ModelRc<CanData>,
        frame_id: u32,
        signal_data: HashMap<String, f32>,
        raw_can: String,
    ) {
        for (message_count, message) in messages.iter().enumerate() {
            if message.can_id == format!("{:08X}", frame_id) {
                let now = Utc::now().timestamp_micros();
                let can_signals = Self::create_can_signals(&message, &signal_data);
                let circle_time =
                    (now - (message.time_stamp).parse::<i64>().unwrap()) as f32 / 1000.0;
                messages.set_row_data(
                    message_count,
                    CanData {
                        can_id: message.can_id.clone(),
                        packet_name: message.packet_name.clone(),
                        signal_value: can_signals.into(),
                        counter: message.counter + 1,
                        raw_can: raw_can.into(),
                        color: if message_count % 2 == 0 {
                            EVEN_COLOR
                        } else {
                            ODD_COLOR
                        },
                        circle_time: format!("{:.02} ms", circle_time).into(),
                        time_stamp: now.to_string().into(),
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

    fn array_to_hex_string(data: &[u8]) -> String {
        // Preallocate space for efficiency
        let mut hex_string = String::with_capacity(data.len() * 3);
        for byte in data {
            write!(hex_string, "{:02X} ", byte).unwrap();
        }
        hex_string.pop(); // Remove the trailing space
        hex_string
    }

    fn bitrate(&self) -> Option<u32> {
        let bitrate_map: HashMap<&str, u32> = [
            ("1 Mbit/s", 1_000_000),
            ("800 kbit/s", 800_000),
            ("500 kbit/s", 500_000),
            ("250 kbit/s", 250_000),
            ("125 kbit/s", 125_000),
            ("100 kbit/s", 100_000),
            ("95.238 kbit/s", 95_238),
            ("83.333 kbit/s", 83_333),
            ("50 kbit/s", 50_000),
            ("47.619 kbit/s", 47_619),
            ("33.333 kbit/s", 33_333),
            ("20 kbit/s", 20_000),
            ("10 kbit/s", 10_000),
            ("5 kbit/s", 5_000),
        ]
        .iter()
        .cloned()
        .collect();

        bitrate_map.get(self.bitrate.as_str()).copied()
    }
}
