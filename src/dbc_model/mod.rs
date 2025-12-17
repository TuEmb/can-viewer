//! DBC model module for editing and writing DBC files
//!
//! This module provides mutable data structures for editing DBC files
//! and a writer to serialize them back to the DBC text format.

mod dbc_writer;
mod editable_dbc;

pub use dbc_writer::DbcWriter;
pub use editable_dbc::{EditableDBC, EditableMessage, EditableSignal};
