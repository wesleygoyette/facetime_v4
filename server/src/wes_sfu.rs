use core::error::Error;
use std::{net::SocketAddr, sync::Arc};

use log::{error, info};
use tokio::{
    net::{TcpListener, TcpStream, UdpSocket},
    sync::Mutex,
};

use crate::{tcp_handler::TcpHandler, udp_handler::UdpHandler};

pub struct WeSFU {
    tcp_listener: TcpListener,
    udp_socket: UdpSocket,
    active_usernames: Arc<Mutex<Vec<String>>>,
}

impl WeSFU {
    pub async fn bind(
        tcp_addr: String,
        udp_addr: String,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        return Ok(Self {
            tcp_listener: TcpListener::bind(tcp_addr).await?,
            udp_socket: UdpSocket::bind(udp_addr).await?,
            active_usernames: Arc::new(Mutex::new(Vec::new())),
        });
    }

    pub async fn listen(self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let udp_handler = UdpHandler::new();

        tokio::spawn(async move { udp_handler.handle_socket(self.udp_socket).await });

        loop {
            let (tcp_stream, tcp_addr) = self.tcp_listener.accept().await?;

            Self::spawn_tcp_thread(tcp_stream, tcp_addr, self.active_usernames.clone());
        }
    }

    fn spawn_tcp_thread(
        mut tcp_stream: TcpStream,
        tcp_addr: SocketAddr,
        active_usernames: Arc<Mutex<Vec<String>>>,
    ) {
        tokio::spawn(async move {
            info!("Opened Connection to {}", tcp_addr);

            let tcp_handler = TcpHandler::new(active_usernames);

            if let Err(e) = tcp_handler.handle_stream(&mut tcp_stream).await {
                error!("Error handling connection: {}", e);
            };

            tcp_handler.handle_disconnect_user().await;

            info!("Closed Connection to {}", tcp_addr);
        });
    }
}
