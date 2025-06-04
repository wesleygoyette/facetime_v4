use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(Debug, EnumIter, PartialEq, Eq, Clone)]
pub enum TcpCommandType {
    HelloFromClient,
    HelloFromServer,
    InvalidUsername,
    GetActiveUsers,
    ReturnActiveUsers,
    CreateRoom,
    InvalidRoomName,
    CreateRoomSuccess,
    GetRooms,
    ReturnRooms,
    JoinRoom,
    JoinRoomSuccess,
    InvalidJoinRoom,
    OtherUserJoinedRoom,
    OtherUserLeftRoom,
}

#[derive(PartialEq, Eq)]
pub enum TcpCommandPayloadType {
    None,
    SingleString,
    MultiString,
    StreamID,
    RoomStreamID,
}

impl TcpCommandType {
    pub fn payload_type(&self) -> TcpCommandPayloadType {
        match self {
            TcpCommandType::GetRooms => TcpCommandPayloadType::None,
            TcpCommandType::HelloFromServer => TcpCommandPayloadType::None,
            TcpCommandType::GetActiveUsers => TcpCommandPayloadType::None,
            TcpCommandType::CreateRoomSuccess => TcpCommandPayloadType::None,

            TcpCommandType::CreateRoom => TcpCommandPayloadType::SingleString,
            TcpCommandType::HelloFromClient => TcpCommandPayloadType::SingleString,
            TcpCommandType::InvalidUsername => TcpCommandPayloadType::SingleString,
            TcpCommandType::InvalidRoomName => TcpCommandPayloadType::SingleString,
            TcpCommandType::JoinRoom => TcpCommandPayloadType::SingleString,
            TcpCommandType::InvalidJoinRoom => TcpCommandPayloadType::SingleString,

            TcpCommandType::ReturnRooms => TcpCommandPayloadType::MultiString,
            TcpCommandType::ReturnActiveUsers => TcpCommandPayloadType::MultiString,

            TcpCommandType::JoinRoomSuccess => TcpCommandPayloadType::StreamID,

            TcpCommandType::OtherUserJoinedRoom => TcpCommandPayloadType::RoomStreamID,
            TcpCommandType::OtherUserLeftRoom => TcpCommandPayloadType::RoomStreamID,
        }
    }

    pub fn to_byte(&self) -> u8 {
        TcpCommandType::iter()
            .position(|v| v == *self)
            .map(|i| 69 + i as u8)
            .expect("TcpCommandType not found in iterator — this should be impossible")
    }

    pub fn from_byte(
        command: u8,
    ) -> Result<TcpCommandType, Box<dyn std::error::Error + Send + Sync>> {
        for tcp_command_type in TcpCommandType::iter() {
            if tcp_command_type.to_byte() == command {
                return Ok(tcp_command_type);
            }
        }
        return Err("Failed to parse command".into());
    }
}
