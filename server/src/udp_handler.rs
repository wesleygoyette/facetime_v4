use core::error::Error;
use std::{collections::HashMap, sync::Arc};

use shared::StreamID;
use tokio::{net::UdpSocket, sync::Mutex};

use crate::room::Room;

pub struct UdpHandler {}

impl UdpHandler {
    pub async fn handle_socket(
        socket: UdpSocket,
        sid_to_username_map: Arc<Mutex<HashMap<StreamID, String>>>,
        rooms: Arc<Mutex<Vec<Room>>>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut username_to_socket_addr_map = HashMap::new();
        let mut buf = [0; 1500];

        loop {
            let (n, from_addr) = socket.recv_from(&mut buf).await?;

            if n < StreamID::default().len() + 1 {
                continue;
            }

            let sid_len = StreamID::default().len();

            let sid: StreamID = buf[0..sid_len]
                .try_into()
                .expect("Invalid SID slice length");

            let from_username_option = {
                let guard = sid_to_username_map.lock().await;
                guard.get(&sid).cloned()
            };

            if let Some(from_username) = from_username_option {
                // println!("[UDP] Received id: {:?}, username: {}", sid, from_username);
                username_to_socket_addr_map.insert(from_username.clone(), from_addr);

                let rooms_snapshot = {
                    let guard = rooms.lock().await;
                    guard.clone()
                };

                let current_room_option = rooms_snapshot
                    .iter()
                    .find(|room| room.username_to_rsid.contains_key(&from_username));

                let current_room = match current_room_option {
                    Some(current_room) => current_room,
                    None => continue,
                };

                let rsid = match current_room.username_to_rsid.get(&from_username) {
                    Some(rsid) => rsid,
                    None => continue,
                };

                for to_username in current_room.username_to_rsid.keys() {
                    if to_username == &from_username {
                        continue;
                    }

                    let send_addr = match username_to_socket_addr_map.get(to_username) {
                        Some(to_user_addr) => to_user_addr,
                        None => continue,
                    };

                    let message_bytes = &buf[sid_len..n];

                    let mut payload = Vec::with_capacity(1 + message_bytes.len());
                    payload.extend_from_slice(rsid);
                    payload.extend_from_slice(message_bytes);

                    socket.send_to(&payload, send_addr).await?;
                }
            }
        }
    }
}
