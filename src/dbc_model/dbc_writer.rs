//! DBC file writer - serializes EditableDBC to DBC text format
//!
//! DBC format reference:
//! - VERSION ""
//! - NS_ : (new symbols)
//! - BS_: (bit timing)
//! - BU_: node1 node2 (nodes)
//! - BO_ <id> <name>: <dlc> <transmitter>
//!    SG_ <name> : <start>|<size>@<byte_order><sign> (<factor>,<offset>) [<min>|<max>] "<unit>" <receivers>
//! - CM_ BO_ <id> "comment";
//! - CM_ SG_ <id> <signal_name> "comment";

use super::editable_dbc::{EditableDBC, EditableMessage, EditableSignal};
use can_dbc::{ByteOrder, MultiplexIndicator, ValueType};
use std::fmt::Write;

/// DBC file writer
pub struct DbcWriter;

impl DbcWriter {
    /// Serialize EditableDBC to DBC file format string
    pub fn write(dbc: &EditableDBC) -> String {
        let mut output = String::new();

        Self::write_version(&mut output, &dbc.version);
        Self::write_new_symbols(&mut output);
        Self::write_bit_timing(&mut output);
        Self::write_nodes(&mut output, &dbc.nodes);
        output.push('\n');
        Self::write_messages(&mut output, &dbc.messages);
        Self::write_comments(&mut output, &dbc.messages);

        output
    }

    fn write_version(output: &mut String, version: &str) {
        writeln!(output, "VERSION \"{}\"", version).unwrap();
        output.push('\n');
    }

    fn write_new_symbols(output: &mut String) {
        // Write minimal new symbols section
        writeln!(output, "NS_ :").unwrap();
        output.push('\n');
    }

    fn write_bit_timing(output: &mut String) {
        // Write empty bit timing section
        writeln!(output, "BS_:").unwrap();
        output.push('\n');
    }

    fn write_nodes(output: &mut String, nodes: &[String]) {
        write!(output, "BU_:").unwrap();
        for node in nodes {
            write!(output, " {}", node).unwrap();
        }
        writeln!(output).unwrap();
    }

    fn write_messages(output: &mut String, messages: &[EditableMessage]) {
        for message in messages {
            Self::write_message(output, message);
        }
    }

    fn write_message(output: &mut String, message: &EditableMessage) {
        // Calculate the raw message ID (with extended bit if needed)
        let raw_id = if message.is_extended {
            message.message_id | (1 << 31)
        } else {
            message.message_id
        };

        // BO_ <id> <name>: <dlc> <transmitter>
        writeln!(
            output,
            "BO_ {} {}: {} {}",
            raw_id, message.name, message.message_size, message.transmitter
        )
        .unwrap();

        // Write all signals
        for signal in &message.signals {
            Self::write_signal(output, signal);
        }

        output.push('\n');
    }

    fn write_signal(output: &mut String, signal: &EditableSignal) {
        // SG_ <name> [M|m<n>] : <start>|<size>@<byte_order><sign> (<factor>,<offset>) [<min>|<max>] "<unit>" <receivers>

        // Multiplexer indicator
        let mux_indicator = match signal.multiplexer_indicator {
            MultiplexIndicator::Multiplexor => " M".to_string(),
            MultiplexIndicator::MultiplexedSignal(n) => format!(" m{}", n),
            MultiplexIndicator::MultiplexorAndMultiplexedSignal(n) => format!(" m{} M", n),
            MultiplexIndicator::Plain => String::new(),
        };

        // Byte order: 1 = little-endian, 0 = big-endian
        let byte_order = match signal.byte_order {
            ByteOrder::LittleEndian => "1",
            ByteOrder::BigEndian => "0",
        };

        // Value type: + = unsigned, - = signed
        let value_type = match signal.value_type {
            ValueType::Unsigned => "+",
            ValueType::Signed => "-",
        };

        // Receivers
        let receivers = if signal.receivers.is_empty() {
            "Vector__XXX".to_string()
        } else {
            signal.receivers.join(",")
        };

        writeln!(
            output,
            " SG_ {}{} : {}|{}@{}{} ({},{}) [{}|{}] \"{}\" {}",
            signal.name,
            mux_indicator,
            signal.start_bit,
            signal.signal_size,
            byte_order,
            value_type,
            format_float(signal.factor),
            format_float(signal.offset),
            format_float(signal.min),
            format_float(signal.max),
            signal.unit,
            receivers
        )
        .unwrap();
    }

    fn write_comments(output: &mut String, messages: &[EditableMessage]) {
        let mut has_comments = false;

        for message in messages {
            // Write message comment
            if let Some(comment) = &message.comment {
                if !has_comments {
                    output.push('\n');
                    has_comments = true;
                }
                let raw_id = if message.is_extended {
                    message.message_id | (1 << 31)
                } else {
                    message.message_id
                };
                writeln!(output, "CM_ BO_ {} \"{}\";", raw_id, escape_comment(comment)).unwrap();
            }

            // Write signal comments
            for signal in &message.signals {
                if let Some(comment) = &signal.comment {
                    if !has_comments {
                        output.push('\n');
                        has_comments = true;
                    }
                    let raw_id = if message.is_extended {
                        message.message_id | (1 << 31)
                    } else {
                        message.message_id
                    };
                    writeln!(
                        output,
                        "CM_ SG_ {} {} \"{}\";",
                        raw_id,
                        signal.name,
                        escape_comment(comment)
                    )
                    .unwrap();
                }
            }
        }
    }
}

/// Format a float value for DBC output
/// Removes unnecessary trailing zeros while keeping precision
fn format_float(value: f64) -> String {
    if value == value.trunc() {
        // Integer value, no decimal needed
        format!("{}", value as i64)
    } else {
        // Format with precision, remove trailing zeros
        let formatted = format!("{:.10}", value);
        let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
        trimmed.to_string()
    }
}

/// Escape special characters in comments
fn escape_comment(comment: &str) -> String {
    comment.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use can_dbc::DBC;

    #[test]
    fn test_format_float() {
        assert_eq!(format_float(1.0), "1");
        assert_eq!(format_float(0.1), "0.1");
        assert_eq!(format_float(0.125), "0.125");
        assert_eq!(format_float(255.0), "255");
        assert_eq!(format_float(-40.0), "-40");
    }

    #[test]
    fn test_write_empty_dbc() {
        let dbc = EditableDBC::new();
        let output = DbcWriter::write(&dbc);
        assert!(output.contains("VERSION"));
        assert!(output.contains("NS_ :"));
        assert!(output.contains("BS_:"));
        assert!(output.contains("BU_:"));
    }

    #[test]
    fn test_write_message_with_signal() {
        let mut dbc = EditableDBC::new();
        let mut msg = EditableMessage::new(100, "TestMessage".to_string());
        msg.signals.push(EditableSignal {
            name: "TestSignal".to_string(),
            start_bit: 0,
            signal_size: 8,
            byte_order: ByteOrder::LittleEndian,
            value_type: ValueType::Unsigned,
            factor: 1.0,
            offset: 0.0,
            min: 0.0,
            max: 255.0,
            unit: "unit".to_string(),
            receivers: vec!["Vector__XXX".to_string()],
            multiplexer_indicator: MultiplexIndicator::Plain,
            comment: None,
        });
        dbc.add_message(msg);

        let output = DbcWriter::write(&dbc);
        assert!(output.contains("BO_ 100 TestMessage: 8 Vector__XXX"));
        assert!(output.contains("SG_ TestSignal : 0|8@1+ (1,0) [0|255] \"unit\" Vector__XXX"));
    }

    #[test]
    fn test_round_trip_parse_edit_write_parse() {
        // Original DBC content
        const ORIGINAL_DBC: &str = r#"VERSION ""

NS_ :

BS_:

BU_: ECU1 ECU2

BO_ 256 EngineData: 8 ECU1
 SG_ EngineSpeed : 0|16@1+ (0.25,0) [0|16383.75] "rpm" ECU2
 SG_ EngineTemp : 16|8@1+ (1,-40) [-40|215] "degC" ECU2

BO_ 512 VehicleSpeed: 4 ECU1
 SG_ Speed : 0|16@1+ (0.01,0) [0|655.35] "km/h" ECU2

"#;

        // Step 1: Parse original DBC
        let parsed_dbc = DBC::from_slice(ORIGINAL_DBC.as_bytes())
            .expect("Failed to parse original DBC");

        // Step 2: Convert to editable
        let mut editable = EditableDBC::from_dbc(&parsed_dbc);

        // Verify initial state
        assert_eq!(editable.messages.len(), 2);
        assert_eq!(editable.messages[0].signals.len(), 2);

        // Step 3: Make some edits
        editable.messages[0].name = "ModifiedEngineData".to_string();
        editable.messages[0].signals[0].factor = 0.5;

        // Add a new signal
        editable.messages[0].add_signal(EditableSignal {
            name: "NewSignal".to_string(),
            start_bit: 24,
            signal_size: 8,
            byte_order: ByteOrder::LittleEndian,
            value_type: ValueType::Unsigned,
            factor: 1.0,
            offset: 0.0,
            min: 0.0,
            max: 255.0,
            unit: "count".to_string(),
            receivers: vec!["ECU2".to_string()],
            multiplexer_indicator: MultiplexIndicator::Plain,
            comment: None,
        });

        // Step 4: Write back to DBC format
        let written_dbc = DbcWriter::write(&editable);

        // Verify written output contains our changes
        assert!(written_dbc.contains("ModifiedEngineData"));
        assert!(written_dbc.contains("NewSignal"));
        assert!(written_dbc.contains("(0.5,0)")); // Modified factor

        // Step 5: Parse the written DBC again
        let reparsed_dbc = DBC::from_slice(written_dbc.as_bytes())
            .expect("Failed to parse written DBC");

        // Step 6: Convert back to editable and verify
        let final_editable = EditableDBC::from_dbc(&reparsed_dbc);

        assert_eq!(final_editable.messages.len(), 2);
        assert_eq!(final_editable.messages[0].name, "ModifiedEngineData");
        assert_eq!(final_editable.messages[0].signals.len(), 3);
        assert_eq!(final_editable.messages[0].signals[0].factor, 0.5);

        // Find the new signal
        let new_signal = final_editable.messages[0].signals
            .iter()
            .find(|s| s.name == "NewSignal");
        assert!(new_signal.is_some());
        assert_eq!(new_signal.unwrap().unit, "count");
    }

    #[test]
    fn test_write_with_comments() {
        let mut dbc = EditableDBC::new();
        let mut msg = EditableMessage::new(100, "TestMessage".to_string());
        msg.comment = Some("This is a message comment".to_string());
        msg.signals.push(EditableSignal {
            name: "TestSignal".to_string(),
            start_bit: 0,
            signal_size: 8,
            byte_order: ByteOrder::LittleEndian,
            value_type: ValueType::Unsigned,
            factor: 1.0,
            offset: 0.0,
            min: 0.0,
            max: 255.0,
            unit: "".to_string(),
            receivers: vec!["Vector__XXX".to_string()],
            multiplexer_indicator: MultiplexIndicator::Plain,
            comment: Some("This is a signal comment".to_string()),
        });
        dbc.add_message(msg);

        let output = DbcWriter::write(&dbc);
        assert!(output.contains("CM_ BO_ 100 \"This is a message comment\";"));
        assert!(output.contains("CM_ SG_ 100 TestSignal \"This is a signal comment\";"));
    }

    #[test]
    fn test_write_signed_big_endian_signal() {
        let mut dbc = EditableDBC::new();
        let mut msg = EditableMessage::new(200, "TestMessage".to_string());
        msg.signals.push(EditableSignal {
            name: "SignedBigEndian".to_string(),
            start_bit: 7,
            signal_size: 16,
            byte_order: ByteOrder::BigEndian,
            value_type: ValueType::Signed,
            factor: 0.1,
            offset: -100.0,
            min: -1000.0,
            max: 1000.0,
            unit: "units".to_string(),
            receivers: vec!["ECU1".to_string()],
            multiplexer_indicator: MultiplexIndicator::Plain,
            comment: None,
        });
        dbc.add_message(msg);

        let output = DbcWriter::write(&dbc);
        // @0- means big-endian (0) and signed (-)
        assert!(output.contains("SG_ SignedBigEndian : 7|16@0- (0.1,-100) [-1000|1000] \"units\" ECU1"));
    }
}
