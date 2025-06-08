use crossterm::terminal::{self};

use std::collections::HashMap;

use shared::{RoomStreamID, StreamID, TcpCommandType, read_command_from_tcp_stream};
use tokio::io::{AsyncWriteExt, stdout};
use tokio::net::TcpStream;
use tokio::sync::broadcast;
use tokio::sync::watch;
use tokio::time::{Duration, Instant, sleep};
use tokio::{net::UdpSocket, sync::Mutex};

use std::sync::Arc;

use crate::ascii_converter::{HEIGHT, WIDTH};
use crate::{
    ascii_converter::AsciiConverter,
    camera::{Camera, RealCamera, TestCamera, TestPatten},
};

const MAX_FRAME_RATE: u64 = 30;

pub struct CallHandler {}

impl CallHandler {
    pub async fn handle_call(
        room_name: &str,
        sid: StreamID,
        test_pattern: Option<TestPatten>,
        tcp_stream: &mut TcpStream,
        udp_socket: UdpSocket,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let udp_socket = udp_socket;
        let udp_socket_arc = Arc::new(udp_socket);

        println!("Joining {}...", room_name);

        println!("Starting camera ASCII feed... Press Ctrl+C to exit");
        let mut camera: Box<dyn Camera> = match test_pattern {
            Some(test_camera_type) => Box::new(TestCamera::new(WIDTH, HEIGHT, test_camera_type)?),
            None => Box::new(RealCamera::new()?),
        };
        println!("Camera initialized successfully!");

        let (current_frame_tx, mut current_frame_rx) = watch::channel(Vec::new());

        let socket_receiver = udp_socket_arc.clone();
        let socket_sender = udp_socket_arc;

        let (task_ender_tx, mut send_task_ender) = broadcast::channel(1);
        let mut recv_task_ender = send_task_ender.resubscribe();

        let send_task = tokio::spawn(async move {
            let mut udp_payload = Vec::with_capacity(1500);

            loop {
                tokio::select! {

                    _ = send_task_ender.recv() => {
                        return;
                    }

                    _ = async {
                        let start_time = Instant::now();

                        udp_payload.clear();
                        udp_payload.extend_from_slice(&sid);

                        let frame = match camera.get_frame() {
                            Ok(f) => f,
                            Err(e) => {
                                eprintln!("Failed to get frame: {}", e);
                                return;
                            }
                        };

                        let frame_bytes = match AsciiConverter::frame_to_nibbles(frame) {
                            Ok(fb) => fb,
                            Err(e) => {
                                eprintln!("Failed to convert frame: {}", e);
                                return;
                            }
                        };

                        udp_payload.extend_from_slice(&frame_bytes);

                        if let Err(e) = current_frame_tx.send(frame_bytes) {
                            eprintln!("Error sending to current_frame_tx: {}", e);
                            return;
                        }

                        if let Err(_) = socket_sender.send(&udp_payload).await {
                            return;
                        }

                        let target_frame_duration = Duration::from_millis(1000 / MAX_FRAME_RATE);
                        let elapsed = start_time.elapsed();

                        if elapsed < target_frame_duration {
                            sleep(target_frame_duration - elapsed).await;
                        }
                    } => {}
                }
            }
        });

        let woppa_dopaa: Arc<Mutex<HashMap<RoomStreamID, Vec<u8>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let woppa_dopaa_clone = woppa_dopaa.clone();

        let recv_task = tokio::spawn({
            async move {
                let mut buf = [0u8; 1500];

                loop {
                    tokio::select! {

                        _ = recv_task_ender.recv() => {
                            return;
                        }

                        result = socket_receiver.recv(&mut buf) => {
                            match result {
                                Ok(n) => {
                                    let user_stream_id = buf[0];
                                    let frame_from_network_bytes = &buf[1..n];
                                    let mut guard = woppa_dopaa.lock().await;
                                        if let Some(x) = guard.get_mut(&[user_stream_id]) {
                                            *x = Vec::from(frame_from_network_bytes);
                                        }
                                }
                                Err(_) => {
                                    continue;
                                }
                            }
                        }
                    }
                }
            }
        });

        loop {
            tokio::select! {

                result = read_command_from_tcp_stream(tcp_stream) => {

                    let command = match result? {
                        Some(command) => command,
                        None => {
                            break;
                        },
                    };

                    match command {
                        shared::TcpCommand::WithRoomStreamIDPayload {command_type: TcpCommandType::OtherUserJoinedRoom, payload } => {

                            let rsid = payload;

                            woppa_dopaa_clone.lock().await.insert(rsid, vec![]);

                        },
                        shared::TcpCommand::WithRoomStreamIDPayload {command_type: TcpCommandType::OtherUserLeftRoom, payload } => {

                            let rsid = payload;

                            woppa_dopaa_clone.lock().await.remove(&rsid);

                        },
                        _ => {}
                    }
                }

                result = current_frame_rx.changed() => {

                    result?;

                    let start_time = Instant::now();

                    let mut all_frames = Vec::new();

                    let current_frame = current_frame_rx.borrow().clone();

                    all_frames.push(current_frame);

                    let other_frames_snapshot: Vec<Vec<u8>> = {
                        let guard = woppa_dopaa_clone.lock().await;
                        guard.values().cloned().collect()
                    };

                    all_frames.extend(other_frames_snapshot.iter().map(|s| s.clone()));

                    AsciiConverter::clear_terminal();
                    let (width, height) = terminal::size()?;
                    print_frames(all_frames, width - 1, height - 1);
                    stdout().flush().await?;

                    let target_frame_duration = Duration::from_millis(1000 / MAX_FRAME_RATE);

                    let elapsed = start_time.elapsed();

                    if elapsed < target_frame_duration {
                        sleep(target_frame_duration - elapsed).await;
                    }
                }
            }
        }

        let _ = task_ender_tx.send(());

        recv_task.await?;
        send_task.await?;

        return Ok(());
    }
}

fn print_frames(frames: Vec<Vec<u8>>, width: u16, height: u16) {
    match frames.len() {
        1 => {
            let my_nibbles = &frames[0];

            print!(
                "{}",
                AsciiConverter::nibbles_to_ascii(my_nibbles, width, height)
            );
        }
        2 => {
            let my_nibbles = &frames[0];
            let your_nibbles = &frames[1];

            if width as f64 * 0.38f64 < height as f64 {
                println!(
                    "{}",
                    AsciiConverter::nibbles_to_ascii(my_nibbles, width, height / 2)
                );
                print!(
                    "{}",
                    AsciiConverter::nibbles_to_ascii(your_nibbles, width, height / 2)
                );
            } else {
                print_two_ascii_frames_side_by_side(
                    &AsciiConverter::nibbles_to_ascii(my_nibbles, width / 2, height),
                    &AsciiConverter::nibbles_to_ascii(your_nibbles, width / 2, height),
                )
            }
        }
        len => {
            let num_rows = ((len + 1) / 2) as u16;

            let frame_height = height / num_rows;
            let frame_width = width / 2;

            let ascii_frames: Vec<String> = frames
                .iter()
                .map(|f| AsciiConverter::nibbles_to_ascii(f, frame_width, frame_height))
                .collect();

            for pair in ascii_frames.chunks(2) {
                if pair.len() == 2 {
                    print_two_ascii_frames_side_by_side(&pair[0], &pair[1]);
                } else {
                    println!("{}", pair[0]);
                }
            }
        }
    }
}

pub fn print_two_ascii_frames_side_by_side(frame1: &str, frame2: &str) {
    let frame1_lines: Vec<&str> = frame1.lines().collect();
    let frame2_lines: Vec<&str> = frame2.lines().collect();

    let max_lines = frame1_lines.len().max(frame2_lines.len());

    for i in 0..max_lines {
        let line1 = frame1_lines.get(i).copied().unwrap_or("");
        let line2 = frame2_lines.get(i).copied().unwrap_or("");

        println!("{} {}", line1, line2);
    }
}

// fn print_frames_side_by_side(frames: Vec<String>) {
// let mut frame_pairs = frames.chunks(2);

// while let Some(pair) = frame_pairs.next() {
//     let frame1_lines: Vec<&str> = pair[0].lines().collect();
//     let frame2_lines: Vec<&str> = if pair.len() == 2 {
//         pair[1].lines().collect()
//     } else {
//         Vec::new()
//     };

//     let max_lines = frame1_lines.len().max(frame2_lines.len());

//     for i in 0..max_lines {
//         let line1 = frame1_lines.get(i).unwrap_or(&"");
//         let line2 = frame2_lines.get(i).unwrap_or(&"");
//         println!("{} {}", line1, line2);
//     }

//     println!();
// }
// }
