//! Mutable DBC data structures for editing
//!
//! These structures mirror the can_dbc crate's types but are fully mutable,
//! allowing for editing and saving DBC files.

use can_dbc::{ByteOrder, MessageId, MultiplexIndicator, ValueType, DBC};

/// Mutable signal structure for editing
#[derive(Clone, Debug)]
pub struct EditableSignal {
    pub name: String,
    pub start_bit: u64,
    pub signal_size: u64,
    pub byte_order: ByteOrder,
    pub value_type: ValueType,
    pub factor: f64,
    pub offset: f64,
    pub min: f64,
    pub max: f64,
    pub unit: String,
    pub receivers: Vec<String>,
    pub multiplexer_indicator: MultiplexIndicator,
    pub comment: Option<String>,
}

impl Default for EditableSignal {
    fn default() -> Self {
        Self {
            name: String::new(),
            start_bit: 0,
            signal_size: 8,
            byte_order: ByteOrder::LittleEndian,
            value_type: ValueType::Unsigned,
            factor: 1.0,
            offset: 0.0,
            min: 0.0,
            max: 255.0,
            unit: String::new(),
            receivers: vec!["Vector__XXX".to_string()],
            multiplexer_indicator: MultiplexIndicator::Plain,
            comment: None,
        }
    }
}

/// Mutable message structure for editing
#[derive(Clone, Debug, Default)]
pub struct EditableMessage {
    pub message_id: u32,
    pub is_extended: bool,
    pub name: String,
    pub message_size: u64,
    pub transmitter: String,
    pub signals: Vec<EditableSignal>,
    pub comment: Option<String>,
}

impl EditableMessage {
    /// Create a new message with default values
    pub fn new(id: u32, name: String) -> Self {
        Self {
            message_id: id,
            is_extended: false,
            name,
            message_size: 8,
            transmitter: "Vector__XXX".to_string(),
            signals: Vec::new(),
            comment: None,
        }
    }

    /// Add a new signal to this message
    pub fn add_signal(&mut self, signal: EditableSignal) {
        self.signals.push(signal);
    }

    /// Remove a signal by index
    pub fn remove_signal(&mut self, index: usize) -> Option<EditableSignal> {
        if index < self.signals.len() {
            Some(self.signals.remove(index))
        } else {
            None
        }
    }

    /// Get a mutable reference to a signal by index
    pub fn get_signal_mut(&mut self, index: usize) -> Option<&mut EditableSignal> {
        self.signals.get_mut(index)
    }

    /// Get the MessageId enum for this message
    pub fn get_message_id(&self) -> MessageId {
        if self.is_extended {
            MessageId::Extended(self.message_id)
        } else {
            MessageId::Standard(self.message_id as u16)
        }
    }
}

/// Mutable DBC structure for editing
#[derive(Clone, Debug, Default)]
pub struct EditableDBC {
    pub version: String,
    pub nodes: Vec<String>,
    pub messages: Vec<EditableMessage>,
}

impl EditableDBC {
    /// Create a new empty DBC
    pub fn new() -> Self {
        Self {
            version: String::new(),
            nodes: Vec::new(),
            messages: Vec::new(),
        }
    }

    /// Convert from can_dbc::DBC to EditableDBC
    pub fn from_dbc(dbc: &DBC) -> Self {
        let version = dbc.version().0.clone();

        let nodes: Vec<String> = dbc
            .nodes()
            .iter()
            .flat_map(|node| node.0.clone())
            .collect();

        let messages: Vec<EditableMessage> = dbc
            .messages()
            .iter()
            .map(|msg| {
                let (message_id, is_extended) = match msg.message_id() {
                    MessageId::Standard(id) => (*id as u32, false),
                    MessageId::Extended(id) => (*id, true),
                };

                let transmitter = match msg.transmitter() {
                    can_dbc::Transmitter::NodeName(name) => name.clone(),
                    can_dbc::Transmitter::VectorXXX => "Vector__XXX".to_string(),
                };

                // Get message comment if available
                let comment = dbc.message_comment(*msg.message_id()).map(|s| s.to_string());

                let signals: Vec<EditableSignal> = msg
                    .signals()
                    .iter()
                    .map(|sig| {
                        // Get signal comment if available
                        let sig_comment = dbc
                            .signal_comment(*msg.message_id(), sig.name())
                            .map(|s| s.to_string());

                        EditableSignal {
                            name: sig.name().clone(),
                            start_bit: sig.start_bit,
                            signal_size: sig.signal_size,
                            byte_order: *sig.byte_order(),
                            value_type: *sig.value_type(),
                            factor: sig.factor,
                            offset: sig.offset,
                            min: sig.min,
                            max: sig.max,
                            unit: sig.unit().clone(),
                            receivers: sig.receivers().clone(),
                            multiplexer_indicator: *sig.multiplexer_indicator(),
                            comment: sig_comment,
                        }
                    })
                    .collect();

                EditableMessage {
                    message_id,
                    is_extended,
                    name: msg.message_name().clone(),
                    message_size: *msg.message_size(),
                    transmitter,
                    signals,
                    comment,
                }
            })
            .collect();

        Self {
            version,
            nodes,
            messages,
        }
    }

    /// Add a new message
    pub fn add_message(&mut self, message: EditableMessage) {
        self.messages.push(message);
    }

    /// Remove a message by index
    pub fn remove_message(&mut self, index: usize) -> Option<EditableMessage> {
        if index < self.messages.len() {
            Some(self.messages.remove(index))
        } else {
            None
        }
    }

    /// Get a mutable reference to a message by index
    pub fn get_message_mut(&mut self, index: usize) -> Option<&mut EditableMessage> {
        self.messages.get_mut(index)
    }

    /// Find the next available message ID
    pub fn next_available_id(&self) -> u32 {
        let max_id = self
            .messages
            .iter()
            .map(|m| m.message_id)
            .max()
            .unwrap_or(0);
        max_id + 1
    }

    /// Validate the DBC structure
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Check for duplicate message IDs
        let mut seen_ids = std::collections::HashSet::new();
        for msg in &self.messages {
            if !seen_ids.insert((msg.message_id, msg.is_extended)) {
                errors.push(format!(
                    "Duplicate message ID: 0x{:X} ({})",
                    msg.message_id,
                    if msg.is_extended { "extended" } else { "standard" }
                ));
            }
        }

        // Check for empty message names
        for (i, msg) in self.messages.iter().enumerate() {
            if msg.name.is_empty() {
                errors.push(format!("Message {} has empty name", i));
            }

            // Check for duplicate signal names within a message
            let mut seen_signals = std::collections::HashSet::new();
            for sig in &msg.signals {
                if !seen_signals.insert(&sig.name) {
                    errors.push(format!(
                        "Duplicate signal name '{}' in message '{}'",
                        sig.name, msg.name
                    ));
                }
            }

            // Check signal bit positions
            for sig in &msg.signals {
                let max_bit = (msg.message_size * 8) as u64;
                if sig.start_bit + sig.signal_size > max_bit {
                    errors.push(format!(
                        "Signal '{}' in message '{}' exceeds message size (start_bit={}, size={}, max={})",
                        sig.name, msg.name, sig.start_bit, sig.signal_size, max_bit
                    ));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl EditableSignal {
    /// Create a new signal with default values
    pub fn new(name: String) -> Self {
        Self {
            name,
            start_bit: 0,
            signal_size: 8,
            byte_order: ByteOrder::LittleEndian,
            value_type: ValueType::Unsigned,
            factor: 1.0,
            offset: 0.0,
            min: 0.0,
            max: 255.0,
            unit: String::new(),
            receivers: vec!["Vector__XXX".to_string()],
            multiplexer_indicator: MultiplexIndicator::Plain,
            comment: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_DBC: &str = r#"VERSION ""

NS_ :

BS_:

BU_: ECU1 ECU2

BO_ 256 EngineData: 8 ECU1
 SG_ EngineSpeed : 0|16@1+ (0.25,0) [0|16383.75] "rpm" ECU2
 SG_ EngineTemp : 16|8@1+ (1,-40) [-40|215] "degC" ECU2

BO_ 512 VehicleSpeed: 4 ECU1
 SG_ Speed : 0|16@1+ (0.01,0) [0|655.35] "km/h" ECU2

CM_ BO_ 256 "Engine data message";
CM_ SG_ 256 EngineSpeed "Engine rotational speed";
"#;

    #[test]
    fn test_from_dbc_parses_messages() {
        let dbc = DBC::from_slice(SAMPLE_DBC.as_bytes()).expect("Failed to parse DBC");
        let editable = EditableDBC::from_dbc(&dbc);

        assert_eq!(editable.messages.len(), 2);
        assert_eq!(editable.messages[0].name, "EngineData");
        assert_eq!(editable.messages[0].message_id, 256);
        assert_eq!(editable.messages[1].name, "VehicleSpeed");
        assert_eq!(editable.messages[1].message_id, 512);
    }

    #[test]
    fn test_from_dbc_parses_signals() {
        let dbc = DBC::from_slice(SAMPLE_DBC.as_bytes()).expect("Failed to parse DBC");
        let editable = EditableDBC::from_dbc(&dbc);

        let engine_msg = &editable.messages[0];
        assert_eq!(engine_msg.signals.len(), 2);

        let speed_sig = &engine_msg.signals[0];
        assert_eq!(speed_sig.name, "EngineSpeed");
        assert_eq!(speed_sig.start_bit, 0);
        assert_eq!(speed_sig.signal_size, 16);
        assert_eq!(speed_sig.factor, 0.25);
        assert_eq!(speed_sig.unit, "rpm");
    }

    #[test]
    fn test_from_dbc_parses_comments() {
        let dbc = DBC::from_slice(SAMPLE_DBC.as_bytes()).expect("Failed to parse DBC");
        let editable = EditableDBC::from_dbc(&dbc);

        let engine_msg = &editable.messages[0];
        assert_eq!(engine_msg.comment, Some("Engine data message".to_string()));

        let speed_sig = &engine_msg.signals[0];
        assert_eq!(speed_sig.comment, Some("Engine rotational speed".to_string()));
    }

    #[test]
    fn test_add_remove_message() {
        let mut dbc = EditableDBC::default();
        assert_eq!(dbc.messages.len(), 0);

        dbc.add_message(EditableMessage::new(100, "TestMsg".to_string()));
        assert_eq!(dbc.messages.len(), 1);
        assert_eq!(dbc.messages[0].name, "TestMsg");

        dbc.remove_message(0);
        assert_eq!(dbc.messages.len(), 0);
    }

    #[test]
    fn test_add_remove_signal() {
        let mut msg = EditableMessage::new(100, "TestMsg".to_string());
        assert_eq!(msg.signals.len(), 0);

        msg.add_signal(EditableSignal::new("Signal1".to_string()));
        msg.add_signal(EditableSignal::new("Signal2".to_string()));
        assert_eq!(msg.signals.len(), 2);

        msg.remove_signal(0);
        assert_eq!(msg.signals.len(), 1);
        assert_eq!(msg.signals[0].name, "Signal2");
    }

    #[test]
    fn test_next_available_id() {
        let mut dbc = EditableDBC::default();
        assert_eq!(dbc.next_available_id(), 1);

        dbc.add_message(EditableMessage::new(100, "Msg1".to_string()));
        assert_eq!(dbc.next_available_id(), 101);

        dbc.add_message(EditableMessage::new(500, "Msg2".to_string()));
        assert_eq!(dbc.next_available_id(), 501);
    }

    #[test]
    fn test_validate_detects_duplicate_ids() {
        let mut dbc = EditableDBC::default();
        dbc.add_message(EditableMessage::new(100, "Msg1".to_string()));
        dbc.add_message(EditableMessage::new(100, "Msg2".to_string()));

        let result = dbc.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("Duplicate message ID")));
    }

    #[test]
    fn test_validate_detects_empty_name() {
        let mut dbc = EditableDBC::default();
        dbc.add_message(EditableMessage::new(100, "".to_string()));

        let result = dbc.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("empty name")));
    }
}
