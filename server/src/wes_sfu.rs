use core::error::Error;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use log::{error, info};
use shared::StreamID;
use tokio::{
    net::{TcpListener, TcpStream, UdpSocket},
    sync::Mutex,
};

use crate::{room::Room, tcp_handler::TcpHandler, udp_handler::UdpHandler};

pub struct WeSFU {
    tcp_listener: TcpListener,
    udp_socket: UdpSocket,
    active_usernames: Arc<Mutex<Vec<String>>>,
    public_rooms: Arc<Mutex<Vec<Room>>>,
    sid_to_username_map: Arc<Mutex<HashMap<StreamID, String>>>,
}

impl WeSFU {
    pub async fn bind(
        tcp_addr: String,
        udp_addr: String,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let defualt_room = Room {
            name: "def".to_string(),
            users: Vec::new(),
        };

        return Ok(Self {
            tcp_listener: TcpListener::bind(tcp_addr).await?,
            udp_socket: UdpSocket::bind(udp_addr).await?,
            active_usernames: Arc::new(Mutex::new(Vec::new())),
            public_rooms: Arc::new(Mutex::new(vec![defualt_room])),
            sid_to_username_map: Arc::new(Mutex::new(HashMap::new())),
        });
    }

    pub async fn listen(self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let udp_sid_to_username_map = self.sid_to_username_map.clone();
        let tcp_sid_to_username_map = self.sid_to_username_map.clone();

        let udp_public_rooms = self.public_rooms.clone();
        let tcp_public_rooms = self.public_rooms.clone();

        let mut udp_task = tokio::spawn(async move {
            let _ = UdpHandler::handle_socket(
                self.udp_socket,
                udp_sid_to_username_map,
                udp_public_rooms,
            )
            .await;
        });

        loop {
            tokio::select! {

                result = self.tcp_listener.accept() => {

                    let (tcp_stream, tcp_addr) = result?;
                    Self::spawn_tcp_thread(
                        tcp_stream,
                        tcp_addr,
                        self.active_usernames.clone(),
                        tcp_public_rooms.clone(),
                        tcp_sid_to_username_map.clone(),
                    );
                }

                result = &mut udp_task => {

                    if let Err(e) = result{
                        return Err(format!("Error handling UDP: {}. Exiting...", e).into());
                    }
                    return Ok(());
                }
            }
        }
    }

    fn spawn_tcp_thread(
        mut tcp_stream: TcpStream,
        tcp_addr: SocketAddr,
        active_usernames: Arc<Mutex<Vec<String>>>,
        public_rooms: Arc<Mutex<Vec<Room>>>,
        sid_to_username_map: Arc<Mutex<HashMap<StreamID, String>>>,
    ) {
        tokio::spawn(async move {
            info!("Opened Connection to {}", tcp_addr);

            let tcp_handler = TcpHandler::new(active_usernames, public_rooms, sid_to_username_map);

            if let Err(e) = tcp_handler.handle_stream(&mut tcp_stream).await {
                error!("Error handling connection: {}", e);
            };

            tcp_handler.handle_disconnect_user().await;

            info!("Closed Connection to {}", tcp_addr);
        });
    }
}
