#![allow(dead_code)]
use std::{
    iter::once,
    ops::{Deref, DerefMut},
};
use bytes::{BufMut, BytesMut};

pub struct Message(BytesMut);

impl Message {
  pub fn new() -> Self {
    Self(BytesMut::zeroed(12))
  }
  pub fn from(data: &[u8]) -> Self {
    Self(BytesMut::from(data))
  }
  pub fn id(&self) -> u16 {
    u16::from_be_bytes([self[0], self[1]])
  }
  pub fn set_id(&mut self, id: u16) {
    self[..2].copy_from_slice(&id.to_be_bytes());
  }
  pub fn set_query(&mut self) {
    self[2] &= 0b0111_1111;
  }
  pub fn set_response(&mut self) {
    self[2] |= 0b1000_0000;
  }
  pub fn opcode(&self) -> u8 {
    self[2] << 1 >> 4
  }
  pub fn rd(&self) -> u8 {
    self[2] & 0b0000_0001
  }
  pub fn rcode(&self) -> u8 {
    self[3] & 0b0000_1111
  }
  pub fn set_rcode(&mut self, rcode: u8) {
    self[3] = (self[3] & 0b1111_0000) | (0b0000_1111 & rcode)
  }

  pub fn add_question(&mut self, name: &str, record_type: u16, class: u16) {
    self.unsplit(encode_domain(name));
    self.put_u16(record_type);
    self.put_u16(class);
    let question_count = u16::from_be_bytes([self[4], self[5]]) + 1;
    self[4..6].copy_from_slice(&question_count.to_be_bytes());
  }

  pub fn add_answer(&mut self, name: &str, record_type: u16, class: u16, ttl: u32, data: u32) {
    self.unsplit(encode_domain(name));
    self.put_u16(record_type);
    self.put_u16(class);
    self.put_u32(ttl);
    self.put_u16(4u16);
    self.put_u32(data);
    let answer_count = u16::from_be_bytes([self[6], self[7]]) + 1;
    self[6..8].copy_from_slice(&answer_count.to_be_bytes());
  }
}

impl Deref for Message {
  type Target = BytesMut;
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl DerefMut for Message {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

pub fn encode_domain(name: &str) -> BytesMut {
  name.split('.').flat_map(|label| once(label.len() as u8).chain(label.bytes())).chain(once(0u8)).collect()
}