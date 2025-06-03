use log::{error, info};
use shared::{TCP_PORT, UDP_PORT};
use wes_sfu::WeSFU;

mod room;
mod tcp_handler;
mod udp_handler;
mod wes_sfu;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let main_addr = "0.0.0.0";

    let tcp_addr = format!("{}:{}", main_addr, TCP_PORT);
    let udp_addr = format!("{}:{}", main_addr, UDP_PORT);

    let server = match WeSFU::bind(tcp_addr, udp_addr).await {
        Ok(wes_sfu_server) => wes_sfu_server,
        Err(e) => {
            error!("Error binding: {}", e);
            return;
        }
    };

    info!("WeSFU listening on TCP: {}, UDP: {}", TCP_PORT, UDP_PORT);

    match server.listen().await {
        Ok(()) => (),
        Err(e) => {
            error!("{}", e);
            return;
        }
    };
}
