use core::error::Error;
use tokio::io::AsyncWriteExt;
use tokio::io::{self, AsyncBufReadExt, BufReader};

use shared::{
    TcpCommand, TcpCommandType, read_command_from_tcp_stream, write_command_to_tcp_stream,
};
use tokio::net::TcpStream;

use crate::user_input_handler::{UserCommand, UserInputHandler};

pub struct Client {
    tcp_stream: TcpStream,
    username: String,
}

const PROMPT: &str = "> ";

impl Client {
    pub async fn connect(addr: &str, username: &str) -> Result<Self, Box<dyn Error>> {
        let mut tcp_stream = TcpStream::connect(addr).await?;

        let hello_command = TcpCommand::WithStringPayload {
            command_type: shared::TcpCommandType::HelloFromClient,
            payload: username.to_string(),
        };

        write_command_to_tcp_stream(hello_command, &mut tcp_stream).await?;

        let resonse_command = match read_command_from_tcp_stream(&mut tcp_stream).await? {
            Some(command) => command,
            None => return Err("Server closed the connection".into()),
        };

        let username = match resonse_command {
            TcpCommand::Simple(TcpCommandType::HelloFromServer) => username.to_string(),
            TcpCommand::WithStringPayload {
                command_type: TcpCommandType::InvalidUsername,
                payload,
            } => return Err(payload.into()),
            _ => return Err("Server sent invalid response".into()),
        };

        return Ok(Self {
            tcp_stream,
            username,
        });
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            tokio::select! {

                result = read_line(PROMPT) => {
                    let line = result?;
                    match UserInputHandler::handle(&line).await? {
                        UserCommand::Close => {
                            println!("Exiting...");
                            return Ok(());
                        },
                        UserCommand::ListUsers => {

                            write_command_to_tcp_stream(TcpCommand::Simple(TcpCommandType::GetActiveUsers), &mut self.tcp_stream).await?;
                            let command_option = read_command_from_tcp_stream(&mut self.tcp_stream).await?;

                            let command = match command_option {
                                Some(cmd) => cmd,
                                None => return Err("Connection closed by the server".into()),
                            };

                            let active_users = match command {
                                TcpCommand::WithMultiStringPayload { command_type: TcpCommandType::ReturnActiveUsers, payload } => payload,
                                _ =>
                                    return Err("Invalid response from server".into())

                            };

                            let total_string = active_users.len().to_string();
                            println!("\n╔══════════════════════════════════╗");
                            println!("║ Active users {}(total: {}) ║", " ".repeat(10 - total_string.len()), total_string);
                            println!("╠══════════════════════════════════╣");
                            for user in active_users {

                                if user == self.username {

                                    println!("║ • {:30} ║", user + " (you)");
                                }
                                else {
                                    println!("║ • {:30} ║", user);
                                }
                            }
                            println!("╚══════════════════════════════════╝\n");
                        },
                        UserCommand::KeepAlive => continue,
                    }

                }

                result = read_command_from_tcp_stream(&mut self.tcp_stream) => {

                    let command_option = result?;

                    let command = match command_option {
                        Some(command) => command,
                        None => return Ok(()),
                    };

                    match command {
                        _ => return Err("Unkown command received".into())
                    }
                }
            }
        }
    }
}

pub async fn read_line(prompt: &str) -> io::Result<String> {
    let mut stdout = io::stdout();
    stdout.write_all(prompt.as_bytes()).await?;
    stdout.flush().await?;

    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin).lines();

    if let Some(line) = reader.next_line().await? {
        return Ok(line.trim().to_string());
    } else {
        return Ok(String::new());
    }
}
