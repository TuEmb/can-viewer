use slint::Weak;
use slint::{Model, VecModel};
use std::rc::Rc;

use crate::slint_generatedAppWindow::AppWindow;
use crate::slint_generatedAppWindow::CanData;
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
            list_filter.push(self.filter);
        } else {
            // Remove filter ID
            for (filter_count, can_filter) in list_filter.clone().into_iter().enumerate() {
                if can_filter.can_id == self.filter.can_id {
                    list_filter.remove(filter_count);
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
