use core::error::Error;
use std::sync::Arc;

use log::info;
use shared::{
    TcpCommand, TcpCommandType, read_command_from_tcp_stream, write_command_to_tcp_stream,
};
use tokio::{net::TcpStream, sync::Mutex};

pub struct TcpHandler {
    current_username: Arc<Mutex<Option<String>>>,
    active_usernames: Arc<Mutex<Vec<String>>>,
}

impl TcpHandler {
    pub fn new(active_usernames: Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            current_username: Arc::new(Mutex::new(None)),
            active_usernames,
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
