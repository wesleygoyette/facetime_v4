pub const TCP_PORT: u16 = 8069;
pub const UDP_PORT: u16 = 8070;

mod tcp_command;
mod tcp_command_type;

pub use tcp_command::TcpCommand;
pub use tcp_command::read_command_from_tcp_stream;
pub use tcp_command::write_command_to_tcp_stream;
pub use tcp_command_type::TcpCommandType;
