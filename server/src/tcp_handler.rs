use std::{collections::HashMap, sync::Arc};

use log::{error, info};
use rand::{Rng, rng};
use shared::{
    RoomStreamID, StreamID, TcpCommand, TcpCommandType, read_command_from_tcp_stream,
    write_command_to_tcp_stream,
};
use tokio::{
    net::TcpStream,
    sync::{Mutex, broadcast},
};

use crate::room::Room;

pub struct TcpHandler {
    current_username: Arc<Mutex<Option<String>>>,
    active_usernames: Arc<Mutex<Vec<String>>>,
    public_rooms: Arc<Mutex<Vec<Room>>>,
    sid_to_username_map: Arc<Mutex<HashMap<StreamID, String>>>,
    username_to_command_channel_tx: Arc<Mutex<HashMap<String, broadcast::Sender<TcpCommand>>>>,
}

impl TcpHandler {
    pub async fn new(
        active_usernames: Arc<Mutex<Vec<String>>>,
        public_rooms: Arc<Mutex<Vec<Room>>>,
        sid_to_username_map: Arc<Mutex<HashMap<StreamID, String>>>,
        username_to_command_channel_tx: Arc<Mutex<HashMap<String, broadcast::Sender<TcpCommand>>>>,
    ) -> Self {
        let current_username = Arc::new(Mutex::new(None));

        Self {
            current_username,
            active_usernames,
            public_rooms,
            sid_to_username_map,
            username_to_command_channel_tx,
        }
    }

    pub async fn handle_stream(
        &mut self,
        stream: &mut TcpStream,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
                    payload: "Username must contain only letters, numbers, underscores (_), or hyphens (-)."
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
            tokio::select! {

                result = read_command_from_tcp_stream(stream) => {

                    let command_option = result?;

                    let command = match command_option {
                        Some(command) => command,
                        None => return Ok(()),
                    };

                    let started_call = self.handle_command_from_user(command, stream).await?;

                    if started_call {
                        break;
                    }
                }
            }
        }

        let tcp_command_channel_rx = match self
            .username_to_command_channel_tx
            .lock()
            .await
            .get(&current_username)
        {
            Some(tx) => tx.subscribe(),
            None => {
                return Err(format!(
                    "Could not find tcp_command_channel_rx for user: {}",
                    current_username
                )
                .into());
            }
        };

        self.handle_call_stream(stream, tcp_command_channel_rx)
            .await?;

        return Ok(());
    }

    async fn handle_call_stream(
        &self,
        stream: &mut TcpStream,
        mut tcp_command_channel_rx: broadcast::Receiver<TcpCommand>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        loop {
            tokio::select! {

                result = read_command_from_tcp_stream(stream) => {

                    if result? == None {
                        return Ok(());
                    }
                }

                result = tcp_command_channel_rx.recv() => {

                    let command = result?;

                    write_command_to_tcp_stream(command, stream).await?;
                }
            }
        }
    }

    pub async fn handle_command_from_user(
        &self,
        command: TcpCommand,
        stream: &mut TcpStream,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        match command {
            TcpCommand::Simple(TcpCommandType::GetActiveUsers) => {
                let active_usernames: Vec<String> =
                    self.active_usernames.lock().await.iter().cloned().collect();

                let response_command = TcpCommand::WithMultiStringPayload {
                    command_type: TcpCommandType::ReturnActiveUsers,
                    payload: active_usernames,
                };

                write_command_to_tcp_stream(response_command, stream).await?;
                return Ok(false);
            }
            TcpCommand::WithStringPayload {
                command_type: TcpCommandType::CreateRoom,
                payload,
            } => {
                let room_name = payload;

                let current_username = match self.current_username.lock().await.clone() {
                    Some(current_username) => current_username,
                    None => return Err("Invalid user when creating room".into()),
                };

                if !is_valid_room_name(&room_name) {
                    let response_command = TcpCommand::WithStringPayload {
                        command_type: TcpCommandType::InvalidRoomName,
                        payload:
                            "Room name must contain only letters, numbers, underscores (_), or hyphens (-)."
                                .to_string(),
                    };

                    write_command_to_tcp_stream(response_command, stream).await?;
                    return Ok(false);
                }

                if room_name.len() > 20 {
                    let response_command = TcpCommand::WithStringPayload {
                        command_type: TcpCommandType::InvalidRoomName,
                        payload: "Room name must be less than or equal to 20 characters."
                            .to_string(),
                    };

                    write_command_to_tcp_stream(response_command, stream).await?;
                    return Ok(false);
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
                    return Ok(false);
                }

                public_rooms_guard.push(Room {
                    name: room_name.clone(),
                    username_to_rsid: HashMap::new(),
                });

                info!("{} created room: {}", current_username, room_name);

                let response_command = TcpCommand::Simple(TcpCommandType::CreateRoomSuccess);
                write_command_to_tcp_stream(response_command, stream).await?;

                return Ok(false);
            }

            TcpCommand::WithStringPayload {
                command_type: TcpCommandType::DeleteRoom,
                payload,
            } => {
                let room_name = payload;

                let current_username = match self.current_username.lock().await.clone() {
                    Some(current_username) => current_username,
                    None => return Err("Invalid user when creating room".into()),
                };

                let mut public_rooms = self.public_rooms.lock().await;

                let room_contains_users = public_rooms
                    .iter()
                    .any(|r| r.name == room_name && r.username_to_rsid.len() > 0);

                if room_contains_users {
                    let response_command = TcpCommand::WithStringPayload {
                        command_type: TcpCommandType::InvalidRoomName,
                        payload: format!(
                            "Room '{}' is in use and cannot be deleted at this time.",
                            room_name
                        )
                        .to_string(),
                    };

                    write_command_to_tcp_stream(response_command, stream).await?;
                    return Ok(false);
                }

                let before_len = public_rooms.len();
                public_rooms.retain(|r| r.name != room_name);
                let after_len = public_rooms.len();

                let rooms_deleted = before_len - after_len;

                if rooms_deleted == 0 {
                    let response_command = TcpCommand::WithStringPayload {
                        command_type: TcpCommandType::InvalidRoomName,
                        payload: format!("Room: '{}' does not exist.", room_name).to_string(),
                    };

                    write_command_to_tcp_stream(response_command, stream).await?;
                    return Ok(false);
                }

                info!("{} deleted room: {}", current_username, room_name);

                let response_command = TcpCommand::Simple(TcpCommandType::DeleteRoomSuccess);
                write_command_to_tcp_stream(response_command, stream).await?;

                return Ok(false);
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
                return Ok(false);
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

                    let rsid: RoomStreamID = rng().random();

                    room.username_to_rsid.insert(current_username.clone(), rsid);

                    for user in room.username_to_rsid.keys() {
                        if user == &current_username {
                            continue;
                        }

                        let username_to_command_channel_tx_guard =
                            self.username_to_command_channel_tx.lock().await;
                        let tx_option = username_to_command_channel_tx_guard.get(user);

                        if let Some(tx) = tx_option {
                            let command = TcpCommand::WithRoomStreamIDPayload {
                                command_type: TcpCommandType::OtherUserJoinedRoom,
                                payload: rsid,
                            };

                            if let Err(e) = tx.send(command) {
                                error!("Error sending to channel: {} for user: {}", e, user);
                            }
                        }
                    }

                    info!("{} joined room: {}", current_username, room.name);

                    let response_command = TcpCommand::WithStreamIDPayload {
                        command_type: TcpCommandType::JoinRoomSuccess,
                        payload: sid,
                    };
                    write_command_to_tcp_stream(response_command, stream).await?;

                    for (user, rsid) in room.username_to_rsid.iter() {
                        if user != &current_username {
                            let add_user_command = TcpCommand::WithRoomStreamIDPayload {
                                command_type: TcpCommandType::OtherUserJoinedRoom,
                                payload: *rsid,
                            };
                            write_command_to_tcp_stream(add_user_command, stream).await?;
                        }
                    }

                    return Ok(true);
                } else {
                    let response_command = TcpCommand::WithStringPayload {
                        command_type: TcpCommandType::InvalidJoinRoom,
                        payload: "Room not found".to_string(),
                    };
                    write_command_to_tcp_stream(response_command, stream).await?;

                    return Ok(false);
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

        let tx = broadcast::Sender::new(16);

        self.username_to_command_channel_tx
            .lock()
            .await
            .insert(current_username.to_string(), tx);

        info!("{} is connected", current_username);
    }

    pub async fn handle_disconnect_user(&self) {
        if let Some(current_username) = self.current_username.lock().await.take() {
            let mut active_usernames_guard = self.active_usernames.lock().await;
            active_usernames_guard.retain(|x| *x != current_username);

            self.username_to_command_channel_tx
                .lock()
                .await
                .remove(&current_username);

            for room in self.public_rooms.lock().await.iter_mut() {
                if let Some(rsid) = room.username_to_rsid.get(&current_username) {
                    for user in room.username_to_rsid.keys() {
                        if user == &current_username {
                            continue;
                        }

                        let username_to_command_channel_tx_guard =
                            self.username_to_command_channel_tx.lock().await;
                        let tx_option = username_to_command_channel_tx_guard.get(user);

                        if let Some(tx) = tx_option {
                            let command = TcpCommand::WithRoomStreamIDPayload {
                                command_type: TcpCommandType::OtherUserLeftRoom,
                                payload: *rsid,
                            };

                            if let Err(e) = tx.send(command) {
                                error!("Error sending to channel: {}", e);
                            }
                        }
                    }
                }

                if let Some(_) = room.username_to_rsid.remove(&current_username) {
                    info!("{} left room: {}", current_username, room.name);
                }
            }

            info!("{} disconnected", current_username);
        }
    }
}

fn is_valid_username(username: &str) -> bool {
    username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn is_valid_room_name(room_name: &str) -> bool {
    room_name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}
