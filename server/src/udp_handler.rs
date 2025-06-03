use core::error::Error;

use tokio::net::UdpSocket;

pub struct UdpHandler {}

impl UdpHandler {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn handle_socket(
        &self,
        socket: UdpSocket,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut buf = [0; 1500];

        loop {
            let (n, from_addr) = socket.recv_from(&mut buf).await?;

            println!("Received {} bytes from {}", n, from_addr);
        }
    }
}
