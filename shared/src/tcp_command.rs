use core::error::Error;
use std::{io::ErrorKind, str::from_utf8};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::{
    StreamID,
    tcp_command_type::{TcpCommandPayloadType, TcpCommandType},
};

#[derive(Debug)]
pub enum TcpCommand {
    Simple(TcpCommandType),
    WithStringPayload {
        command_type: TcpCommandType,
        payload: String,
    },
    WithMultiStringPayload {
        command_type: TcpCommandType,
        payload: Vec<String>,
    },
    WithStreamIDPayload {
        command_type: TcpCommandType,
        payload: StreamID,
    },
}

impl TcpCommand {
    pub fn get_command_type(&self) -> TcpCommandType {
        match self {
            TcpCommand::Simple(command_type) => command_type.clone(),
            TcpCommand::WithStringPayload { command_type, .. } => command_type.clone(),
            TcpCommand::WithMultiStringPayload { command_type, .. } => command_type.clone(),
            TcpCommand::WithStreamIDPayload { command_type, .. } => command_type.clone(),
        }
    }
}

pub async fn read_command_from_tcp_stream(
    tcp_stream: &mut TcpStream,
) -> Result<Option<TcpCommand>, Box<dyn Error>> {
    let mut command_type_buf = [0; 1];
    loop {
        match tcp_stream.read(&mut command_type_buf).await {
            Ok(0) => return Ok(None),
            Ok(_) => break,
            Err(e) if e.kind() == ErrorKind::WouldBlock => continue,
            Err(e) => {
                return Err(e.into());
            }
        }
    }

    let command_type = TcpCommandType::from_byte(command_type_buf[0])?;

    match command_type.payload_type() {
        TcpCommandPayloadType::None => {
            let command = TcpCommand::Simple(command_type);

            return Ok(Some(command));
        }
        TcpCommandPayloadType::SingleString => {
            let mut payload_len_buf = [0; 1];
            tcp_stream.read_exact(&mut payload_len_buf).await?;

            let payload_len = payload_len_buf[0] as usize;

            let mut payload_buf = vec![0; payload_len];
            tcp_stream.read_exact(&mut payload_buf).await?;

            let payload = String::from_utf8(payload_buf)?;

            let command = TcpCommand::WithStringPayload {
                command_type,
                payload,
            };

            return Ok(Some(command));
        }
        TcpCommandPayloadType::MultiString => {
            let mut payload_len_buf = [0; 1];
            tcp_stream.read_exact(&mut payload_len_buf).await?;

            let payload_len = payload_len_buf[0] as usize;

            let mut payload = vec![];

            for _ in 0..payload_len {
                let mut payload_string_len_buf = [0; 1];
                tcp_stream.read_exact(&mut payload_string_len_buf).await?;
                let payload_string_len = payload_string_len_buf[0] as usize;

                let mut payload_string_bytes = vec![0; payload_string_len];
                tcp_stream.read_exact(&mut payload_string_bytes).await?;

                let payload_string = from_utf8(&payload_string_bytes)?.to_string();

                payload.push(payload_string);
            }

            let command = TcpCommand::WithMultiStringPayload {
                command_type,
                payload,
            };

            return Ok(Some(command));
        }

        TcpCommandPayloadType::StreamID => {
            let mut payload = StreamID::default();
            tcp_stream.read_exact(&mut payload).await?;

            let command = TcpCommand::WithStreamIDPayload {
                command_type,
                payload,
            };

            return Ok(Some(command));
        }
    }
}

pub async fn write_command_to_tcp_stream(
    command: TcpCommand,
    tcp_stream: &mut TcpStream,
) -> Result<(), Box<dyn Error>> {
    match command {
        TcpCommand::Simple(command_type) => {
            if command_type.payload_type() != TcpCommandPayloadType::None {
                return Err("Incorrect payload type".into());
            }

            let message = &[command_type.to_byte()];
            tcp_stream.write_all(message).await?;
        }
        TcpCommand::WithStringPayload {
            command_type,
            payload,
        } => {
            if command_type.payload_type() != TcpCommandPayloadType::SingleString {
                return Err("Incorrect payload type".into());
            }

            if payload.len() > u8::MAX as usize {
                return Err("Command payload too long".into());
            }

            let mut message = vec![command_type.to_byte()];
            message.push(payload.len() as u8);
            message.extend(payload.as_bytes());

            tcp_stream.write_all(&message).await?;
        }
        TcpCommand::WithMultiStringPayload {
            command_type,
            payload,
        } => {
            if command_type.payload_type() != TcpCommandPayloadType::MultiString {
                return Err("Incorrect payload type".into());
            }

            if payload.len() > u8::MAX as usize {
                return Err("Command payload too long".into());
            }

            let mut message = vec![command_type.to_byte()];
            message.push(payload.len() as u8);
            for payload_string in payload {
                if payload_string.len() > u8::MAX as usize {
                    return Err("Command payload string too long".into());
                }

                message.push(payload_string.len() as u8);
                message.extend(payload_string.as_bytes());
            }

            tcp_stream.write_all(&message).await?;
        }

        TcpCommand::WithStreamIDPayload {
            command_type,
            payload,
        } => {
            if command_type.payload_type() != TcpCommandPayloadType::StreamID {
                return Err("Incorrect payload type".into());
            }

            let mut message = vec![command_type.to_byte()];
            message.extend(payload);

            tcp_stream.write_all(&message).await?;
        }
    }

    return Ok(());
}
