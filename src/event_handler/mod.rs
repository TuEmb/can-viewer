pub(crate) mod can_handler;
pub(crate) mod dbc_file;
pub(crate) mod packet_filter;

pub use can_handler::CanHandler;
pub use dbc_file::DBCFile;
pub use packet_filter::PacketFilter;
#[cfg(target_os = "windows")]
use pcan_basic::socket::Baudrate;
use slint::Color;

const ODD_COLOR: Color = Color::from_rgb_u8(0x18, 0x1c, 0x27);
const EVEN_COLOR: Color = Color::from_rgb_u8(0x13, 0x16, 0x1f);

#[cfg(target_os = "windows")]
pub fn p_can_bitrate(bitrate: &str) -> Option<Baudrate> {
    match bitrate {
        "1 Mbit/s" => Some(Baudrate::Baud1M),
        "800 kbit/s" => Some(Baudrate::Baud800K),
        "500 kbit/s" => Some(Baudrate::Baud500K),
        "250 kbit/s" => Some(Baudrate::Baud250K),
        "125 kbit/s" => Some(Baudrate::Baud125K),
        "100 kbit/s" => Some(Baudrate::Baud100K),
        "95.238 kbit/s" => Some(Baudrate::Baud95K),
        "83.333 kbit/s" => Some(Baudrate::Baud83),
        "50 kbit/s" => Some(Baudrate::Baud50K),
        "47.619 kbit/s" => Some(Baudrate::Baud47K),
        "33.333 kbit/s" => Some(Baudrate::Baud33K),
        "20 kbit/s" => Some(Baudrate::Baud20K),
        "10 kbit/s" => Some(Baudrate::Baud10K),
        "5 kbit/s" => Some(Baudrate::Baud5K),
        _ => None,
    }
}
