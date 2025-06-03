use core::error::Error;

use shared::{StreamID, UDP_PORT};
use tokio::{net::UdpSocket, time::Duration, time::sleep};

use crate::{
    ascii_converter::AsciiConverter,
    camera::{Camera, RealCamera, TestCamera, TestPatten},
};

const WIDTH: i32 = 90;
const HEIGHT: i32 = 28;

pub struct CallHandler {}

impl CallHandler {
    pub async fn handle_call(
        room_name: &str,
        sid: StreamID,
        test_pattern: Option<TestPatten>,
    ) -> Result<(), Box<dyn Error>> {
        println!("Joining {}...", room_name);

        let client_bind_addr = "0.0.0.0:0";
        let server_udp_addr = format!("127.0.0.1:{}", UDP_PORT);

        let socket = UdpSocket::bind(&client_bind_addr).await?;
        socket.connect(&server_udp_addr).await?;

        let mut camera: Box<dyn Camera> = match test_pattern {
            Some(test_camera_type) => Box::new(TestCamera::new(WIDTH, HEIGHT, test_camera_type)?),
            None => Box::new(RealCamera::new(WIDTH, HEIGHT)?),
        };

        let mut buf = [0; 1500];

        loop {
            tokio::select! {

                _ = sleep(Duration::from_millis(10)) => {

                    let mut udp_payload = Vec::new();
                    udp_payload.extend(sid);

                    let frame = camera.get_frame()?;
                    let frame_bytes = AsciiConverter::ascii_frame_to_bytes(frame);

                    udp_payload.extend(frame_bytes);

                    socket.send(&udp_payload).await?;
                }
                result = socket.recv(&mut buf) => {

                    let message_len = result?;

                    let message_bytes = &buf[0..message_len];
                    let message = AsciiConverter::bytes_to_ascii_frame(message_bytes);

                    AsciiConverter::clear_terminal();
                    println!("{}", message);
                }
            }
        }
    }
}
