use core::error::Error;
use std::{collections::HashMap, sync::Arc};

use log::info;
use rand::{Rng, rng};
use shared::{
    StreamID, TcpCommand, TcpCommandType, read_command_from_tcp_stream, write_command_to_tcp_stream,
};
use tokio::{net::TcpStream, sync::Mutex};

use crate::room::Room;

pub struct TcpHandler {
    current_username: Arc<Mutex<Option<String>>>,
    active_usernames: Arc<Mutex<Vec<String>>>,
    public_rooms: Arc<Mutex<Vec<Room>>>,
    sid_to_username_map: Arc<Mutex<HashMap<StreamID, String>>>,
}

impl TcpHandler {
    pub fn new(
        active_usernames: Arc<Mutex<Vec<String>>>,
        public_rooms: Arc<Mutex<Vec<Room>>>,
        sid_to_username_map: Arc<Mutex<HashMap<StreamID, String>>>,
    ) -> Self {
        let current_username = Arc::new(Mutex::new(None));
        Self {
            current_username,
            active_usernames,
            public_rooms,
            sid_to_username_map,
        }
    }

    pub async fn handle_stream(&self, stream: &mut TcpStream) -> Result<(), Box<dyn Error>> {
        let first_command_from_client_option = read_command_from_tcp_stream(stream).await?;

        let potential_username = match first_command_from_client_option {
            Some(TcpCommand::WithStringPayload {
                command_type: TcpCommandType::HelloFromClient,
                payload,
            }) => payload,
            _ => {
                return Err("Expected HelloFromClient command with payload".into());
            }
        };

        if !is_valid_username(&potential_username) {
            write_command_to_tcp_stream(
                TcpCommand::WithStringPayload {
                    command_type: TcpCommandType::InvalidUsername,
                    payload: "Username must contain only alphanumeric characters (A-Z, a-z, 0-9)."
                        .to_string(),
                },
                stream,
            )
            .await?;

            info!("Client sent invalid username");

            return Ok(());
        }

        if potential_username.len() > 20 {
            write_command_to_tcp_stream(
                TcpCommand::WithStringPayload {
                    command_type: TcpCommandType::InvalidUsername,
                    payload: "Username must be less than or equal to 20 characters.".to_string(),
                },
                stream,
            )
            .await?;

            info!("Client sent invalid username");

            return Ok(());
        }

        if self
            .active_usernames
            .lock()
            .await
            .contains(&potential_username)
        {
            write_command_to_tcp_stream(
                TcpCommand::WithStringPayload {
                    command_type: TcpCommandType::InvalidUsername,
                    payload: format!("Username '{}' is already taken.", potential_username),
                },
                stream,
            )
            .await?;

            info!("Client sent invalid username");

            return Ok(());
        }

        write_command_to_tcp_stream(TcpCommand::Simple(TcpCommandType::HelloFromServer), stream)
            .await?;

        let current_username = potential_username;

        self.handle_connect_user(&current_username).await;

        loop {
            let command_option = read_command_from_tcp_stream(stream).await?;

            let command = match command_option {
                Some(cmd) => cmd,
                None => return Ok(()),
            };

            self.handle_command_from_user(command, stream).await?;
        }
    }

    pub async fn handle_command_from_user(
        &self,
        command: TcpCommand,
        stream: &mut TcpStream,
    ) -> Result<(), Box<dyn Error>> {
        match command {
            TcpCommand::Simple(TcpCommandType::GetActiveUsers) => {
                let active_usernames: Vec<String> =
                    self.active_usernames.lock().await.iter().cloned().collect();

                let response_command = TcpCommand::WithMultiStringPayload {
                    command_type: TcpCommandType::ReturnActiveUsers,
                    payload: active_usernames,
                };

                write_command_to_tcp_stream(response_command, stream).await?;
                return Ok(());
            }
            TcpCommand::WithStringPayload {
                command_type: TcpCommandType::CreateRoom,
                payload,
            } => {
                let room_name = payload;

                if !is_valid_room_name(&room_name) {
                    let response_command = TcpCommand::WithStringPayload {
                        command_type: TcpCommandType::InvalidRoomName,
                        payload:
                            "Room name must contain only alphanumeric characters (A-Z, a-z, 0-9)."
                                .to_string(),
                    };

                    write_command_to_tcp_stream(response_command, stream).await?;
                    return Ok(());
                }

                if room_name.len() > 20 {
                    let response_command = TcpCommand::WithStringPayload {
                        command_type: TcpCommandType::InvalidRoomName,
                        payload: "Room name must be less than or equal to 20 characters."
                            .to_string(),
                    };

                    write_command_to_tcp_stream(response_command, stream).await?;
                    return Ok(());
                }

                let mut public_rooms_guard = self.public_rooms.lock().await;

                let room_name_is_taken =
                    public_rooms_guard.iter().any(|room| room.name == room_name);

                if room_name_is_taken {
                    let response_command = TcpCommand::WithStringPayload {
                        command_type: TcpCommandType::InvalidRoomName,
                        payload: format!("Room: '{}' already exists.", room_name).to_string(),
                    };

                    write_command_to_tcp_stream(response_command, stream).await?;
                    return Ok(());
                }

                public_rooms_guard.push(Room {
                    name: room_name,
                    users: Vec::new(),
                });

                let response_command = TcpCommand::Simple(TcpCommandType::CreateRoomSuccess);
                write_command_to_tcp_stream(response_command, stream).await?;

                return Ok(());
            }

            TcpCommand::Simple(TcpCommandType::GetRooms) => {
                let room_names = self
                    .public_rooms
                    .lock()
                    .await
                    .iter()
                    .map(|room| room.name.clone())
                    .collect();

                let response_command = TcpCommand::WithMultiStringPayload {
                    command_type: TcpCommandType::ReturnRooms,
                    payload: room_names,
                };

                write_command_to_tcp_stream(response_command, stream).await?;
                return Ok(());
            }

            TcpCommand::WithStringPayload {
                command_type: TcpCommandType::JoinRoom,
                payload,
            } => {
                let room_name = payload;

                let mut rooms = self.public_rooms.lock().await;

                if let Some(room) = rooms.iter_mut().find(|room| room.name == room_name) {
                    let mut sid = rng().random();
                    let mut try_count = 0;
                    while self.sid_to_username_map.lock().await.contains_key(&sid) {
                        sid = rng().random();

                        if try_count > 10000 {
                            return Err(
                                "Failed to assign SSID within a reasonable time frame".into()
                            );
                        }
                        try_count += 1;
                    }

                    let current_username = {
                        let guard = self.current_username.lock().await;
                        guard
                            .clone()
                            .ok_or_else(|| "Could not find username when assigning StreamID")?
                    };

                    self.sid_to_username_map
                        .lock()
                        .await
                        .insert(sid, current_username.clone());

                    room.users.push(current_username);

                    let response_command = TcpCommand::WithStreamIDPayload {
                        command_type: TcpCommandType::JoinRoomSuccess,
                        payload: sid,
                    };
                    write_command_to_tcp_stream(response_command, stream).await?;
                    return Ok(());
                } else {
                    let response_command = TcpCommand::WithStringPayload {
                        command_type: TcpCommandType::InvalidJoinRoom,
                        payload: "Room not found".to_string(),
                    };
                    write_command_to_tcp_stream(response_command, stream).await?;

                    return Ok(());
                }
            }

            _ => return Err(format!("Command not handled {:?}", command).into()),
        }
    }

    pub async fn handle_connect_user(&self, current_username: &str) {
        let mut current_username_guard = self.current_username.lock().await;
        *current_username_guard = Some(current_username.to_string());

        let mut active_usernames_guard = self.active_usernames.lock().await;
        active_usernames_guard.push(current_username.to_string());

        info!("{} is connected", current_username);
    }

    pub async fn handle_disconnect_user(&self) {
        if let Some(current_username) = self.current_username.lock().await.take() {
            let mut active_usernames_guard = self.active_usernames.lock().await;
            active_usernames_guard.retain(|x| *x != current_username);

            info!("{} disconnected", current_username);
        }
    }
}

fn is_valid_username(username: &str) -> bool {
    username.chars().all(|c| c.is_ascii_alphanumeric())
}

fn is_valid_room_name(room_name: &str) -> bool {
    room_name.chars().all(|c| c.is_ascii_alphanumeric())
}
