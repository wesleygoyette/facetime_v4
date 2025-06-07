use tokio::io::AsyncWriteExt;
use tokio::io::{self, AsyncBufReadExt, BufReader};

use shared::{
    TCP_PORT, TcpCommand, TcpCommandType, UDP_PORT, read_command_from_tcp_stream,
    write_command_to_tcp_stream,
};
use tokio::net::{TcpStream, UdpSocket};

use crate::call_handler::CallHandler;
use crate::camera::TestPatten;
use crate::user_input_handler::{UserCommand, UserInputHandler};

pub struct Client {
    tcp_stream: TcpStream,
    udp_socket_option: Option<UdpSocket>,
    username: String,
}

const PROMPT: &str = "> ";

impl Client {
    pub async fn connect(
        server_addr: &str,
        username: &str,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let server_tcp_addr = format!("{}:{}", server_addr, TCP_PORT);
        let server_udp_addr = format!("{}:{}", server_addr, UDP_PORT);

        let udp_socket = UdpSocket::bind(&"0.0.0.0:0").await?;
        udp_socket.connect(&server_udp_addr).await?;

        let mut tcp_stream = TcpStream::connect(server_tcp_addr).await?;

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
            udp_socket_option: Some(udp_socket),
        });
    }

    pub async fn run(
        &mut self,
        test_pattern: Option<TestPatten>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        loop {
            let line = read_line(PROMPT).await?;
            match UserInputHandler::handle(&line).await? {
                UserCommand::Close => {
                    println!("Exiting...");
                    return Ok(());
                }
                UserCommand::ListUsers => {
                    write_command_to_tcp_stream(
                        TcpCommand::Simple(TcpCommandType::GetActiveUsers),
                        &mut self.tcp_stream,
                    )
                    .await?;
                    let command_option = read_command_from_tcp_stream(&mut self.tcp_stream).await?;

                    let command = match command_option {
                        Some(cmd) => cmd,
                        None => return Err("Connection closed by the server".into()),
                    };

                    let active_users = match command {
                        TcpCommand::WithMultiStringPayload {
                            command_type: TcpCommandType::ReturnActiveUsers,
                            payload,
                        } => payload,
                        _ => return Err("Invalid response from server".into()),
                    };

                    let total_string = active_users.len().to_string();
                    println!("\n╔══════════════════════════════════╗");
                    println!(
                        "║ Active Users {}(total: {}) ║",
                        " ".repeat(10 - total_string.len()),
                        total_string
                    );
                    println!("╠══════════════════════════════════╣");
                    for user in active_users {
                        if user == self.username {
                            println!("║ • {:30} ║", user + " (you)");
                        } else {
                            println!("║ • {:30} ║", user);
                        }
                    }
                    println!("╚══════════════════════════════════╝\n");
                }
                UserCommand::ListRooms => {
                    write_command_to_tcp_stream(
                        TcpCommand::Simple(TcpCommandType::GetRooms),
                        &mut self.tcp_stream,
                    )
                    .await?;
                    let command_option = read_command_from_tcp_stream(&mut self.tcp_stream).await?;

                    let command = match command_option {
                        Some(cmd) => cmd,
                        None => return Err("Connection closed by the server".into()),
                    };

                    let rooms = match command {
                        TcpCommand::WithMultiStringPayload {
                            command_type: TcpCommandType::ReturnRooms,
                            payload,
                        } => payload,
                        _ => return Err("Invalid response from server".into()),
                    };

                    if rooms.len() == 0 {
                        println!("\n╔══════════════════════════════════╗");
                        println!("║ Available Rooms       (total: 0) ║");
                        println!("╚══════════════════════════════════╝\n");
                    } else {
                        let total_string = rooms.len().to_string();
                        println!("\n╔══════════════════════════════════╗");
                        println!(
                            "║ Available Rooms {}(total: {}) ║",
                            " ".repeat(7 - total_string.len()),
                            total_string
                        );
                        println!("╠══════════════════════════════════╣");
                        for room in rooms {
                            println!("║ • {:30} ║", room);
                        }
                        println!("╚══════════════════════════════════╝\n");
                    }
                }
                UserCommand::CreateRoom(room_name) => {
                    let command = TcpCommand::WithStringPayload {
                        command_type: TcpCommandType::CreateRoom,
                        payload: room_name.clone(),
                    };
                    write_command_to_tcp_stream(command, &mut self.tcp_stream).await?;

                    let command_option = read_command_from_tcp_stream(&mut self.tcp_stream).await?;

                    let command = match command_option {
                        Some(cmd) => cmd,
                        None => return Err("Connection closed by the server".into()),
                    };

                    match command {
                        TcpCommand::WithStringPayload {
                            command_type: TcpCommandType::InvalidRoomName,
                            payload,
                        } => {
                            println!("{}", payload);
                        }
                        TcpCommand::Simple(TcpCommandType::CreateRoomSuccess) => {
                            println!("Successfully created room: '{}'", room_name);
                        }
                        _ => return Err("Invalid response from server".into()),
                    };
                }
                UserCommand::DeleteRoom(room_name) => {
                    let command = TcpCommand::WithStringPayload {
                        command_type: TcpCommandType::DeleteRoom,
                        payload: room_name.clone(),
                    };
                    write_command_to_tcp_stream(command, &mut self.tcp_stream).await?;

                    let command_option = read_command_from_tcp_stream(&mut self.tcp_stream).await?;

                    let command = match command_option {
                        Some(cmd) => cmd,
                        None => return Err("Connection closed by the server".into()),
                    };

                    match command {
                        TcpCommand::WithStringPayload {
                            command_type: TcpCommandType::InvalidRoomName,
                            payload,
                        } => {
                            println!("{}", payload);
                        }
                        TcpCommand::Simple(TcpCommandType::DeleteRoomSuccess) => {
                            println!("Successfully deleted room: '{}'", room_name);
                        }
                        _ => return Err("Invalid response from server".into()),
                    };
                }
                UserCommand::JoinRoom(room_name) => {
                    let command = TcpCommand::WithStringPayload {
                        command_type: TcpCommandType::JoinRoom,
                        payload: room_name.clone(),
                    };
                    write_command_to_tcp_stream(command, &mut self.tcp_stream).await?;

                    let command_option = read_command_from_tcp_stream(&mut self.tcp_stream).await?;

                    let command = match command_option {
                        Some(cmd) => cmd,
                        None => return Err("Connection closed by the server".into()),
                    };

                    match command {
                        TcpCommand::WithStreamIDPayload {
                            command_type: TcpCommandType::JoinRoomSuccess,
                            payload,
                        } => {
                            let udp_socket = self
                                .udp_socket_option
                                .take()
                                .expect("UDP socket already taken");

                            CallHandler::handle_call(
                                &room_name,
                                payload,
                                test_pattern,
                                &mut self.tcp_stream,
                                udp_socket,
                            )
                            .await?;
                            return Ok(());
                        }
                        TcpCommand::WithStringPayload {
                            command_type: TcpCommandType::InvalidJoinRoom,
                            payload,
                        } => {
                            println!("{}", payload);
                        }
                        _ => return Err("Invalid response from server".into()),
                    };
                }
                UserCommand::KeepAlive => continue,
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
