use core::error::Error;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(Debug, EnumIter, PartialEq, Eq, Clone)]
pub enum TcpCommandType {
    HelloFromClient,
    HelloFromServer,
    InvalidUsername,
    GetActiveUsers,
    ReturnActiveUsers,
}

#[derive(PartialEq, Eq)]
pub enum TcpCommandPayloadType {
    Byte,
    SingleString,
    MultiString,
}

impl TcpCommandType {
    pub fn payload_type(&self) -> TcpCommandPayloadType {
        match self {
            TcpCommandType::HelloFromServer | TcpCommandType::GetActiveUsers => {
                TcpCommandPayloadType::Byte
            }
            TcpCommandType::HelloFromClient | TcpCommandType::InvalidUsername => {
                TcpCommandPayloadType::SingleString
            }
            TcpCommandType::ReturnActiveUsers => TcpCommandPayloadType::MultiString,
        }
    }

    pub fn to_byte(&self) -> u8 {
        TcpCommandType::iter()
            .position(|v| v == *self)
            .map(|i| 69 + i as u8)
            .expect("TcpCommandType not found in iterator — this should be impossible")
    }

    pub fn from_byte(command: u8) -> Result<TcpCommandType, Box<dyn Error>> {
        for tcp_command_type in TcpCommandType::iter() {
            if tcp_command_type.to_byte() == command {
                return Ok(tcp_command_type);
            }
        }
        return Err("Failed to parse command".into());
    }
}
