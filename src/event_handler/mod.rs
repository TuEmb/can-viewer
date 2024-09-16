pub(crate) mod can_handler;
pub(crate) mod dbc_file;
pub(crate) mod init;
pub(crate) mod packet_filter;

pub use can_handler::CanHandler;
pub use dbc_file::DBCFile;
pub use init::Init;
pub use packet_filter::PacketFilter;
use slint::Color;

const ODD_COLOR: Color = Color::from_rgb_u8(0x18, 0x1c, 0x27);
const EVEN_COLOR: Color = Color::from_rgb_u8(0x13, 0x16, 0x1f);
