use std::{collections::HashMap, rc::Rc, time::Duration};

use crate::slint_generatedAppWindow::{raw_can, AppWindow};
use slint::{Model, SharedString, VecModel, Weak};
use socketcan::{CanSocket, EmbeddedFrame, Frame, Socket};
pub struct DebugHandler<'a> {
    #[cfg(target_os = "linux")]
    pub iface: &'a str,
    #[cfg(target_os = "windows")]
    pub iface: UsbCanSocket,
    pub ui_handle: &'a Weak<AppWindow>,
    pub bitrate: String,
    pub filter: (u32, u32),
}

impl<'a> DebugHandler<'a> {
    pub fn run(&mut self) {
        let can_socket = CanSocket::open(self.iface).unwrap();
        if let Ok(frame) = can_socket.read_frame() {
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
                    vec_data.push(raw_can {
                        data: SharedString::from(format!("{:?}", frame.data())),
                        id: SharedString::from(frame_id.to_string()),
                        len: frame.len() as i32,
                    });
                    vec_data.reverse();
                    let message_vec: Rc<VecModel<raw_can>> = Rc::new(VecModel::from(vec_data));
                    ui.set_raw_data(message_vec.into());
                });
            }
        } else {
            std::thread::sleep(Duration::from_millis(1));
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
