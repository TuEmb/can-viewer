use std::{
    collections::HashMap,
    rc::Rc,
    sync::mpsc::{self, Receiver},
    time::Duration,
};

use crate::slint_generatedAppWindow::{raw_can, AppWindow};
use chrono::Local;
#[cfg(target_os = "windows")]
use pcan_basic::socket::CanFrame;
use slint::{Model, SharedString, VecModel, Weak};
#[cfg(target_os = "linux")]
use socketcan::{CanFrame, EmbeddedFrame, Frame};

const MAX_LEN: usize = 1000;
pub struct DebugHandler<'a> {
    pub ui_handle: &'a Weak<AppWindow>,
    pub bitrate: String,
    pub filter: (u32, u32),
    pub can_rx: Receiver<CanFrame>,
}

impl<'a> DebugHandler<'a> {
    pub fn run(&mut self) {
        let (tx, rx) = mpsc::channel();
        let mut debug_enable = false;
        let tx_clone = tx.clone();
        let _ = self.ui_handle.upgrade_in_event_loop(move |ui| {
            ui.on_change_state(move |state| {
                let _ = tx_clone.send(state);
            });
        });
        loop {
            if let Ok(en) = rx.try_recv() {
                debug_enable = en;
            }
            if debug_enable {
                if let Ok(frame) = self.can_rx.try_recv() {
                    #[cfg(target_os = "windows")]
                    let frame_id = frame.can_id() & !0x80000000;
                    #[cfg(target_os = "linux")]
                    let frame_id = frame.raw_id() & !0x80000000;
                    if frame_id >= self.filter.0 && frame_id <= self.filter.1 {
                        let bitrate = self.bitrate().unwrap();
                        let _ = self.ui_handle.upgrade_in_event_loop(move |ui| {
                            ui.set_bitrate(bitrate as i32);
                            let raw_data = ui.get_raw_data();
                            let mut vec_data = Vec::default();
                            for data in raw_data.iter() {
                                vec_data.push(data);
                            }
                            if vec_data.len() > MAX_LEN {
                                vec_data.remove(MAX_LEN);
                            }
                            vec_data.insert(
                                0,
                                raw_can {
                                    time: SharedString::from(
                                        Local::now().to_string().replace('"', "").to_string(),
                                    ),
                                    data: SharedString::from(format!("{:?}", frame.data())),
                                    id: if frame.is_extended() {
                                        SharedString::from(format!("0x{:08X}", frame_id))
                                    } else {
                                        SharedString::from(format!("0x{:03X}", frame_id))
                                    },
                                    #[cfg(target_os = "linux")]
                                    len: frame.len() as i32,
                                    #[cfg(target_os = "windows")]
                                    len: frame.dlc() as i32,
                                },
                            );
                            let message_vec: Rc<VecModel<raw_can>> =
                                Rc::new(VecModel::from(vec_data));
                            ui.set_raw_data(message_vec.into());
                        });
                    }
                } else {
                    std::thread::sleep(Duration::from_millis(1));
                }
            } else {
                std::thread::sleep(Duration::from_millis(50));
            }
        }
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
