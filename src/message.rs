#![allow(dead_code)]
use std::{
  fmt::Debug,
  iter::once,
  ops::{Deref, DerefMut},
};

use bytes::{BufMut, BytesMut};
use nom::Offset;
use crate::parser::{expand_question, parse_domains};

const HEADER_LENGTH: usize = 12;

pub struct Message(BytesMut);

impl Message {
  pub fn new() -> Self {
    Self(BytesMut::zeroed(HEADER_LENGTH))
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
  pub fn question_count(&self) -> u16 {
    u16::from_be_bytes([self[4], self[5]])
  }
  pub fn set_question_count(&mut self, count: u16) {
    self[4..6].copy_from_slice(&count.to_be_bytes());
  }
  pub fn answer_count(&self) -> u16 {
    u16::from_be_bytes([self[6], self[7]])
  }
  pub fn set_answer_count(&mut self, count: u16) {
    self[6..8].copy_from_slice(&count.to_be_bytes());
  }
  pub fn original_questions(&self) -> Vec<&[u8]> {
    let (_, questions) = parse_domains(&self[HEADER_LENGTH..], self.question_count() as usize).unwrap();
    questions
  }
  
  pub fn expanded_questions(&self) -> Vec<BytesMut> {
    let mut res = Vec::new();
    let mut qc = self.question_count();
    let mut offset = HEADER_LENGTH;
    loop {
      let (r, question) = expand_question(self, offset).unwrap();
      res.push(question);
      qc -= 1;
      if qc == 0 {
        break;
      }
      offset = (self).offset(r);
    }
    res
  }

  pub fn add_question(&mut self, question: &[u8]) {
    assert_eq!(self.answer_count(), 0);
    self.put(question);
    self.set_question_count(self.question_count() + 1);
  }

  pub fn add_answer(&mut self, answer: &[u8]) {
    self.put(answer);
    self.set_answer_count(self.answer_count() + 1);
  }
  
  pub fn answer_question(&mut self, question: &[u8], ttl: u32, data: &[u8]) {
    self.put(question);
    self.put_u32(ttl);
    self.put_u16(data.len() as u16);
    self.put(data);
    let answer_count = u16::from_be_bytes([self[6], self[7]]) + 1;
    self[6..8].copy_from_slice(&answer_count.to_be_bytes());
  }
}

impl From<&[u8]> for Message {
  fn from(value: &[u8]) -> Self {
    Self(BytesMut::from(value))
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

impl Debug for Message {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    self.0.fmt(f)
  }
}

pub fn encode_domain(name: &str) -> BytesMut {
  name.split('.').flat_map(|label| once(label.len() as u8).chain(label.bytes())).chain(once(0u8)).collect()
}