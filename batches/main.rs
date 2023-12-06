use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use bytes::Bytes;
use message::Message;
mod message;

fn handleDataGram(received_data: Bytes, source: SocketAddr, udp_socket: &UdpSocket) {
  eprintln!("received data: {:02X?}", received_data);
  let mut message = Message::from(&received_data);
  message.set_response();
  if message.opcode() == 0 {
    message.set_rcode(0);
  } else {
    message.set_rcode(4);
  }
  message.add_answer("codecrafters.io", 1, 1, 60u32, Ipv4Addr::new(8, 8, 8, 8).into());
  udp_socket.send_to(&message, source).expect("Failed to send response");
}

fn main() {
  let udp_socket = UdpSocket::bind("127.0.0.1:2053").expect("Failed to bind to address");
  let mut buf = [0; 512];
  loop {
    match udp_socket.recv_from(&mut buf) {
      Ok((size, source)) => {
        println!("Received {} bytes from {}", size, source);
        handleDataGram(Bytes::copy_from_slice(&buf[..size]), source, &udp_socket);
      }
      Err(e) => {
        eprintln!("Error receiving data: {}", e);
        break;
      }
    }
  }
}