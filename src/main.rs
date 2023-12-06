use std::{
  env,
  net::{Ipv4Addr, SocketAddr, UdpSocket},
};
use bytes::{Bytes, BytesMut};
use message::Message;
use nom::Offset;
use parser::{expand_answer, expand_question};
mod message;
mod parser;

fn forward_question(message: Message, addr: &str) -> BytesMut {
  let socket = UdpSocket::bind("127.0.0.1:0").expect("failed to bind to a new system assigned port");
  socket.connect(addr).unwrap_or_else(|e| panic!("error connecting to resolver: {e}"));
  socket.send(&message).unwrap_or_else(|e| panic!("error sending message: {e}"));
  let mut buf = [0u8; 512];
  match socket.recv(&mut buf) {
    Ok(size) => {
      let response = &buf[..size];
      let (r, _) = expand_question(response, 12).unwrap_or_else(|e| panic!("expand question error: {e}"));
      let (_, answer) = expand_answer(response, response.offset(r)).unwrap_or_else(|e| panic!("expand answer error: {e}"));
      answer
    }
    Err(e) => panic!("receive from resolver: {e}"),
  }
}

fn handleDataGram(received_data: Bytes, source: SocketAddr, udp_socket: &UdpSocket) {
  eprintln!("received data: {:02X?}", received_data);
  let mut message = Message::from(&received_data[..]);
  let questions = message.expanded_questions();
  if let Some(addr) = env::args().nth(2) {
    for question in questions {
      let mut forward_message = Message::from(&received_data[..12]);
      forward_message.set_question_count(0);
      forward_message.add_question(&question);
      message.add_answer(&forward_question(forward_message, &addr));
    }
  } else {
    for question in questions {
      message.answer_question(&question, 60, &Ipv4Addr::new(8, 8, 8, 8).octets())
    }
  }
  message.set_response();
  if message.opcode() == 0 {
    message.set_rcode(0);
  } else {
    message.set_rcode(4);
  }
  eprintln!("response: {:02X?}", message);
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