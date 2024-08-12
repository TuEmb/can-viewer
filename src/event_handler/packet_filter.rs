use slint::Weak;
use slint::{Model, VecModel};
use std::rc::Rc;

use crate::slint_generatedAppWindow::AppWindow;
use crate::slint_generatedAppWindow::CanData;

use super::{EVEN_COLOR, ODD_COLOR};
pub struct PacketFilter<'a> {
    pub ui_handle: &'a Weak<AppWindow>,
    pub filter: CanData,
    pub is_check: bool,
}

impl<'a> PacketFilter<'a> {
    pub fn process_filter(self) {
        let ui = self.ui_handle.unwrap();
        let mut list_filter: Vec<CanData> = ui.get_filter_messages().iter().collect();
        if self.is_check {
            // Add filter ID
            list_filter.push(CanData {
                can_id: self.filter.can_id,
                color: if list_filter.len() % 2 == 0 {
                    EVEN_COLOR
                } else {
                    ODD_COLOR
                },
                counter: self.filter.counter,
                packet_name: self.filter.packet_name,
                raw_can: self.filter.raw_can,
                signal_value: self.filter.signal_value,
            });
        } else {
            // Remove filter ID
            for (filter_count, can_filter) in list_filter.clone().into_iter().enumerate() {
                if can_filter.can_id == self.filter.can_id {
                    list_filter.remove(filter_count);
                }

                if filter_count % 2 == 0 {
                    if let Some(data) = list_filter.get_mut(filter_count) {
                        data.color = ODD_COLOR;
                    }
                } else if let Some(data) = list_filter.get_mut(filter_count) {
                    data.color = EVEN_COLOR;
                }
            }
        }

        ui.set_filter_messages(Rc::new(VecModel::from(list_filter.clone())).into());

        if list_filter.is_empty() {
            ui.set_is_filter(false);
        } else {
            ui.set_is_filter(true);
        }
    }
}
