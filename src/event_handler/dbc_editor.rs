//! DBC Editor handler - Backend logic for editing DBC files

use crate::dbc_model::{DbcWriter, EditableDBC, EditableMessage, EditableSignal};
use crate::slint_generatedAppWindow::{
    AppWindow, EditableMessageData, EditableSignalData, SignalListItem,
};
use can_dbc::{ByteOrder, ValueType, DBC};
use rfd::FileDialog;
use slint::{SharedString, VecModel, Weak};
use std::cell::RefCell;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::rc::Rc;

/// DBC Editor handler struct
pub struct DbcEditorHandler {
    pub ui_handle: Weak<AppWindow>,
    editable_dbc: Rc<RefCell<Option<EditableDBC>>>,
    current_file_path: Rc<RefCell<Option<PathBuf>>>,
    has_unsaved_changes: Rc<RefCell<bool>>,
}

impl Clone for DbcEditorHandler {
    fn clone(&self) -> Self {
        Self {
            ui_handle: self.ui_handle.clone(),
            editable_dbc: self.editable_dbc.clone(),
            current_file_path: self.current_file_path.clone(),
            has_unsaved_changes: self.has_unsaved_changes.clone(),
        }
    }
}

impl DbcEditorHandler {
    /// Create a new DBC editor handler
    pub fn new(ui_handle: Weak<AppWindow>) -> Self {
        Self {
            ui_handle,
            editable_dbc: Rc::new(RefCell::new(None)),
            current_file_path: Rc::new(RefCell::new(None)),
            has_unsaved_changes: Rc::new(RefCell::new(false)),
        }
    }

    /// Load a DBC file for editing
    pub fn load_dbc_for_edit(&self) {
        if let Some(path) = FileDialog::new()
            .add_filter("DBC files", &["dbc"])
            .set_directory("./")
            .pick_file()
        {
            match self.load_dbc_from_path(&path) {
                Ok(dbc) => {
                    *self.editable_dbc.borrow_mut() = Some(dbc);
                    let path_str = path.to_string_lossy().to_string();
                    *self.current_file_path.borrow_mut() = Some(path);
                    *self.has_unsaved_changes.borrow_mut() = false;
                    self.refresh_ui();
                    self.update_file_path(&path_str);
                }
                Err(e) => {
                    eprintln!("Failed to load DBC: {}", e);
                }
            }
        }
    }

    fn load_dbc_from_path(&self, path: &PathBuf) -> Result<EditableDBC, String> {
        let mut file =
            File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        let dbc = DBC::from_slice(&buffer)
            .map_err(|_| "Failed to parse DBC file".to_string())?;
        Ok(EditableDBC::from_dbc(&dbc))
    }

    /// Save DBC to current file path
    pub fn save_dbc(&self) {
        let path = self.current_file_path.borrow().clone();
        if let Some(path) = path {
            self.save_dbc_to_path(path);
        } else {
            self.save_dbc_as();
        }
    }

    /// Save DBC with file picker
    pub fn save_dbc_as(&self) {
        if let Some(path) = FileDialog::new()
            .add_filter("DBC files", &["dbc"])
            .set_directory("./")
            .save_file()
        {
            self.save_dbc_to_path(path);
        }
    }

    fn save_dbc_to_path(&self, path: PathBuf) {
        if let Some(dbc) = self.editable_dbc.borrow().as_ref() {
            let content = DbcWriter::write(dbc);
            match File::create(&path) {
                Ok(mut file) => {
                    if file.write_all(content.as_bytes()).is_ok() {
                        let path_str = path.to_string_lossy().to_string();
                        *self.current_file_path.borrow_mut() = Some(path);
                        *self.has_unsaved_changes.borrow_mut() = false;
                        self.update_file_path(&path_str);
                        self.update_unsaved_state(false);
                    }
                }
                Err(e) => eprintln!("Failed to save file: {}", e),
            }
        }
    }

    /// Select a message by index
    pub fn select_message(&self, index: i32) {
        if let Some(ui) = self.ui_handle.upgrade() {
            ui.set_editor_selected_message(index);
            ui.set_editor_selected_signal(-1);

            if let Some(dbc) = self.editable_dbc.borrow().as_ref() {
                if let Some(msg) = dbc.messages.get(index as usize) {
                    // Update selected message data
                    let msg_data = self.message_to_ui_data(msg, index as usize);
                    ui.set_editor_selected_message_data(msg_data);

                    // Update signal list for this message
                    let signals: Vec<SignalListItem> = msg
                        .signals
                        .iter()
                        .enumerate()
                        .map(|(i, sig)| SignalListItem {
                            index: i as i32,
                            name: SharedString::from(&sig.name),
                            start_bit: sig.start_bit as i32,
                            size: sig.signal_size as i32,
                            unit: SharedString::from(&sig.unit),
                        })
                        .collect();
                    ui.set_editor_current_signals(Rc::new(VecModel::from(signals)).into());
                }
            }
        }
    }

    /// Select a signal by index
    pub fn select_signal(&self, index: i32) {
        if let Some(ui) = self.ui_handle.upgrade() {
            ui.set_editor_selected_signal(index);

            let msg_index = ui.get_editor_selected_message();
            if let Some(dbc) = self.editable_dbc.borrow().as_ref() {
                if let Some(msg) = dbc.messages.get(msg_index as usize) {
                    if let Some(sig) = msg.signals.get(index as usize) {
                        let sig_data = self.signal_to_ui_data(sig, index as usize);
                        ui.set_editor_selected_signal_data(sig_data);
                    }
                }
            }
        }
    }

    /// Add a new message
    pub fn add_message(&self) {
        let mut dbc_ref = self.editable_dbc.borrow_mut();
        if let Some(dbc) = dbc_ref.as_mut() {
            let new_id = dbc.next_available_id();
            let new_msg = EditableMessage::new(new_id, format!("NewMessage_{}", new_id));
            dbc.add_message(new_msg);
            drop(dbc_ref);

            *self.has_unsaved_changes.borrow_mut() = true;
            self.refresh_ui();
            self.update_unsaved_state(true);
        }
    }

    /// Remove a message by index
    pub fn remove_message(&self, index: i32) {
        let mut dbc_ref = self.editable_dbc.borrow_mut();
        if let Some(dbc) = dbc_ref.as_mut() {
            dbc.remove_message(index as usize);
            drop(dbc_ref);

            *self.has_unsaved_changes.borrow_mut() = true;
            self.refresh_ui();
            self.update_unsaved_state(true);

            // Clear selection
            if let Some(ui) = self.ui_handle.upgrade() {
                ui.set_editor_selected_message(-1);
                ui.set_editor_selected_signal(-1);
            }
        }
    }

    /// Update message properties
    pub fn update_message(&self, message_data: EditableMessageData) {
        let mut dbc_ref = self.editable_dbc.borrow_mut();
        if let Some(dbc) = dbc_ref.as_mut() {
            if let Some(msg) = dbc.get_message_mut(message_data.index as usize) {
                // Parse hex string to message ID
                let id_str = message_data.message_id.trim_start_matches("0x");
                if let Ok(id) = u32::from_str_radix(id_str, 16) {
                    msg.message_id = id;
                }
                msg.is_extended = message_data.is_extended;
                msg.name = message_data.name.to_string();
                msg.message_size = message_data.dlc as u64;

                drop(dbc_ref);
                *self.has_unsaved_changes.borrow_mut() = true;
                self.refresh_ui();
                self.update_unsaved_state(true);
            }
        }
    }

    /// Add a signal to a message
    pub fn add_signal(&self, message_index: i32) {
        let mut dbc_ref = self.editable_dbc.borrow_mut();
        if let Some(dbc) = dbc_ref.as_mut() {
            if let Some(msg) = dbc.get_message_mut(message_index as usize) {
                let sig_name = format!("Signal_{}", msg.signals.len());
                let new_sig = EditableSignal::new(sig_name);
                msg.add_signal(new_sig);

                drop(dbc_ref);
                *self.has_unsaved_changes.borrow_mut() = true;
                // Re-select the message to refresh signal list
                self.select_message(message_index);
                self.refresh_ui();
                self.update_unsaved_state(true);
            }
        }
    }

    /// Remove a signal from a message
    pub fn remove_signal(&self, message_index: i32, signal_index: i32) {
        let mut dbc_ref = self.editable_dbc.borrow_mut();
        if let Some(dbc) = dbc_ref.as_mut() {
            if let Some(msg) = dbc.get_message_mut(message_index as usize) {
                msg.remove_signal(signal_index as usize);

                drop(dbc_ref);
                *self.has_unsaved_changes.borrow_mut() = true;

                // Clear signal selection and refresh
                if let Some(ui) = self.ui_handle.upgrade() {
                    ui.set_editor_selected_signal(-1);
                }
                self.select_message(message_index);
                self.refresh_ui();
                self.update_unsaved_state(true);
            }
        }
    }

    /// Update signal properties
    pub fn update_signal(&self, message_index: i32, signal_data: EditableSignalData) {
        let mut dbc_ref = self.editable_dbc.borrow_mut();
        if let Some(dbc) = dbc_ref.as_mut() {
            if let Some(msg) = dbc.get_message_mut(message_index as usize) {
                if let Some(sig) = msg.get_signal_mut(signal_data.index as usize) {
                    sig.name = signal_data.name.to_string();
                    sig.start_bit = signal_data.start_bit as u64;
                    sig.signal_size = signal_data.signal_size as u64;
                    sig.byte_order = if signal_data.byte_order == 0 {
                        ByteOrder::LittleEndian
                    } else {
                        ByteOrder::BigEndian
                    };
                    sig.value_type = if signal_data.value_type == 0 {
                        ValueType::Unsigned
                    } else {
                        ValueType::Signed
                    };
                    sig.factor = signal_data.factor.parse().unwrap_or(1.0);
                    sig.offset = signal_data.offset.parse().unwrap_or(0.0);
                    sig.min = signal_data.min_val.parse().unwrap_or(0.0);
                    sig.max = signal_data.max_val.parse().unwrap_or(0.0);
                    sig.unit = signal_data.unit.to_string();

                    drop(dbc_ref);
                    *self.has_unsaved_changes.borrow_mut() = true;
                    self.select_message(message_index);
                    self.refresh_ui();
                    self.update_unsaved_state(true);
                }
            }
        }
    }

    /// Refresh the UI with current DBC state
    fn refresh_ui(&self) {
        if let Some(ui) = self.ui_handle.upgrade() {
            if let Some(dbc) = self.editable_dbc.borrow().as_ref() {
                // Convert messages to UI model
                let messages: Vec<EditableMessageData> = dbc
                    .messages
                    .iter()
                    .enumerate()
                    .map(|(i, msg)| self.message_to_ui_data(msg, i))
                    .collect();

                ui.set_editor_messages(Rc::new(VecModel::from(messages)).into());
            }
        }
    }

    fn message_to_ui_data(&self, msg: &EditableMessage, index: usize) -> EditableMessageData {
        EditableMessageData {
            index: index as i32,
            message_id: SharedString::from(format!("0x{:X}", msg.message_id)),
            is_extended: msg.is_extended,
            name: SharedString::from(&msg.name),
            dlc: msg.message_size as i32,
            transmitter: SharedString::from(&msg.transmitter),
            signal_count: msg.signals.len() as i32,
        }
    }

    fn signal_to_ui_data(&self, sig: &EditableSignal, index: usize) -> EditableSignalData {
        EditableSignalData {
            index: index as i32,
            name: SharedString::from(&sig.name),
            start_bit: sig.start_bit as i32,
            signal_size: sig.signal_size as i32,
            byte_order: match sig.byte_order {
                ByteOrder::LittleEndian => 0,
                ByteOrder::BigEndian => 1,
            },
            value_type: match sig.value_type {
                ValueType::Unsigned => 0,
                ValueType::Signed => 1,
            },
            factor: SharedString::from(format!("{}", sig.factor)),
            offset: SharedString::from(format!("{}", sig.offset)),
            min_val: SharedString::from(format!("{}", sig.min)),
            max_val: SharedString::from(format!("{}", sig.max)),
            unit: SharedString::from(&sig.unit),
        }
    }

    fn update_file_path(&self, path: &str) {
        if let Some(ui) = self.ui_handle.upgrade() {
            ui.set_editor_file_path(SharedString::from(path));
        }
    }

    fn update_unsaved_state(&self, unsaved: bool) {
        if let Some(ui) = self.ui_handle.upgrade() {
            ui.set_editor_has_unsaved_changes(unsaved);
        }
    }
}
