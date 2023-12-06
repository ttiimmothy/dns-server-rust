#![allow(dead_code)]
use bytes::{BufMut, BytesMut};
use nom::{
  bits,
  branch::alt,
  bytes::complete::{tag, take},
  character::complete::char as nom_char,
  combinator::{map, value},
  error::Error,
  multi::{count, length_data, many_till},
  number::complete::{be_u16, be_u8},
  sequence::{preceded, terminated},
  IResult, Offset,
};

fn parse_question(i: &[u8]) -> IResult<&[u8], &[u8]> {
  let (r, _) = many_till(length_data(be_u8), nom_char('\0'))(i)?;
  let (r, _) = take(4usize)(r)?;
  Ok((r, &i[..i.offset(r)]))
}

fn parse_questions(i: &[u8], cnt: usize) -> IResult<&[u8], Vec<&[u8]>> {
  count(parse_question, cnt)(i)
}

#[derive(PartialEq, Debug, Clone)]
pub enum DomainPart<'a> {
  Pointer(usize),
  Label(&'a [u8]),
  Terminator,
}

pub fn parse_pointer(i: &[u8]) -> IResult<&[u8], DomainPart> {
  map(
    bits::bits::<_, _, Error<_>, _, _>(preceded(
      bits::complete::tag(0b11, 2usize),
      bits::complete::take(14usize),
    )),
    DomainPart::Pointer,
  )(i)
}

pub fn parse_terminator(i: &[u8]) -> IResult<&[u8], DomainPart> {
  value(DomainPart::Terminator, tag(&[0u8]))(i)
}

pub fn parse_label(i: &[u8]) -> IResult<&[u8], DomainPart> {
  map(
    length_data(bits::bits::<_, usize, Error<_>, _, _>(preceded(
      bits::complete::tag(0b00, 2usize),
      bits::complete::take(6usize),
    ))),
    DomainPart::Label,
  )(i)
}

fn parse_domain(i: &[u8]) -> IResult<&[u8], &[u8]> {
  let (r, (_labels, _ptr_or_ter)) = terminated(
    many_till(parse_label, alt((parse_terminator, parse_pointer))),
    take(4usize),
  )(i)?;
  let offset = i.offset(r);
  Ok((r, &i[..offset]))
}

pub fn parse_domains(i: &[u8], cnt: usize) -> IResult<&[u8], Vec<&[u8]>> {
  count(parse_domain, cnt)(i)
}

pub fn expand_question(i: &[u8], offset: usize) -> IResult<&[u8], BytesMut> {
  let (r, (labels, ptr_or_ter)) = many_till(parse_label, alt((parse_terminator, parse_pointer)))(&i[offset..])?;
  let mut res = BytesMut::new();
  for label in labels {
    let DomainPart::Label(label) = label else {
      continue;
    };
    res.put_u8(label.len() as u8);
    res.put(label);
  }
  match ptr_or_ter {
    DomainPart::Pointer(ptr) => {
      let (_, mut question) = expand_question(i, ptr)?;
      res.put(question.split_to(question.len() - 4));
    }
    DomainPart::Terminator => res.put_u8(0),
    DomainPart::Label(label) => {
      panic!("ptr_or_ter cannot be a label: {label:?}")
    }
  }
  let (r, record_type_class) = take(4usize)(r)?;
  res.put(record_type_class);
  Ok((r, res))
}

pub fn expand_answer(i: &[u8], offset: usize) -> IResult<&[u8], BytesMut> {
  let (r, mut question) = expand_question(i, offset)?;
  let (r, ttl) = take(4usize)(r)?;
  question.put(ttl);
  let (r, length) = be_u16(r)?;
  question.put_u16(length);
  let (r, data) = take(length)(r)?;
  question.put(data);
  Ok((r, question))
}

#[test]
fn test_expand_question() {
  let i = b"\xd7R\x01\0\0\x01\0\0\0\0\0\0\x0ccodecrafters\x02io\0\0\x01\0\x01";
  let (r, question) = expand_question(i, 12).unwrap();
  assert_eq!(r, b"");
  assert_eq!(question.as_ref(), b"\x0ccodecrafters\x02io\0\0\x01\0\x01");
  let i = b"\xfc=\x01\0\0\x02\0\0\0\0\0\0\x03abc\x11longassdomainname\x03com\0\0\x01\0\x01\x03def\xc0\x10\0\x01\0\x01";
  let (r, question) = expand_question(i, 12).unwrap();
  assert_eq!(r, b"\x03def\xc0\x10\0\x01\0\x01");
  assert_eq!(
    question.as_ref(),
    b"\x03abc\x11longassdomainname\x03com\0\0\x01\0\x01"
  );
  let (r, question) = expand_question(i, i.offset(r)).unwrap();
  assert_eq!(r, b"");
  assert_eq!(
    question.as_ref(),
    b"\x03def\x11longassdomainname\x03com\0\0\x01\0\x01"
  );
}

#[test]
fn test_parse_domains() {
  let mut i = Vec::from(b"\x03abc\x02ab\0\0\x01\0\x01");
  i.extend_from_slice(b"\x04zxcv");
  i.extend_from_slice(&((0b11u16 << 14) | 36u16).to_be_bytes());
  i.extend_from_slice(b"\0\x01\0\x01");
  i.extend_from_slice(&((0b11u16 << 14) | 20u16).to_be_bytes());
  i.extend_from_slice(b"\0\x01\0\x01");
  i.push(0u8);
  i.extend_from_slice(b"\0\x01\0\x01");
  i.extend_from_slice(b"hello world");
  let (r, domains) = parse_domains(&i, 4).unwrap();
  assert_eq!(r, b"hello world");
  assert_eq!(domains.len(), 4);
  assert_eq!(domains[0], &i[..12]);
  assert_eq!(domains[1], &i[12..23]);
  assert_eq!(domains[2], &i[23..29]);
  assert_eq!(domains[3], &i[29..34]);
}

#[test]
fn test_parse_domain() {
  let mut i = Vec::from(((0b11u16 << 14) | 20u16).to_be_bytes());
  i.extend_from_slice(b"\0\x01\0\x01hello");
  let (r, domain) = parse_domain(&i).unwrap();
  assert_eq!(r, b"hello");
  assert_eq!(domain, &i[..6]);
  let i = b"\x03abc\x05abcde\0\0\x01\0\x01hello";
  let (r, domain) = parse_domain(i).unwrap();
  assert_eq!(r, b"hello");
  assert_eq!(domain, &i[..15]);
  let mut i = Vec::from(b"\x02ab\x04abcd");
  i.extend_from_slice(&(0b11u16 << 14 | 20u16).to_be_bytes());
  i.extend_from_slice(b"\0\x01\0\x01hello");
  let (r, domain) = parse_domain(&i).unwrap();
  assert_eq!(r, b"hello");
  assert_eq!(domain, &i[..14]);
}

#[test]
fn test_parse_label() {
  let i = b"\x03abc";
  let (r, label) = parse_label(i).unwrap();
  assert!(r.is_empty());
  assert_eq!(label, DomainPart::Label(b"abc"));
}

#[test]
fn test_parse_terminator() {
  let i = b"\0";
  let (r, terminator) = parse_terminator(i).unwrap();
  assert!(r.is_empty());
  assert_eq!(terminator, DomainPart::Terminator);
}

#[test]
fn test_parse_pointer() {
  let i = ((0b11u16 << 14) | (20u16)).to_be_bytes();
  let (r, ptr) = parse_pointer(&i).unwrap();
  assert!(r.is_empty());
  assert_eq!(ptr, DomainPart::Pointer(20));
}

#[test]
fn test_parse_question() {
  let input = b"\x04\xd2\x01\0\0\x01\0\0\0\0\0\0\x0ccodecrafters\x02io\0\0\x01\0\x01";
  let (r, question) = parse_question(&input[12..]).unwrap();
  assert!(r.is_empty());
  assert_eq!(question, b"\x0ccodecrafters\x02io\0\0\x01\0\x01");
}

#[test]
fn test_bits() {
  let i = [0b1110_0101u8, 0b0110_1000];
  let ((r, offset), value) = bits::complete::take::<_, u8, _, Error<_>>(2usize)((&i[..], 0)).unwrap();
  assert_eq!(r, i);
  assert_eq!(offset, 2);
  assert_eq!(value, 0b11u8);
  let ((r, offset), value) = bits::complete::take::<_, u16, _, Error<_>>(9usize)((&i[..], 0)).unwrap();
  assert_eq!(r, &i[1..]);
  assert_eq!(offset, 1);
  assert_eq!(value, 0b1_1100_1010_u16);
  let ((r, offset), value) = bits::complete::tag::<_, u8, _, Error<_>>(0b11, 2usize)((&i[..], 0)).unwrap();
  assert_eq!(r, i);
  assert_eq!(offset, 2);
  assert_eq!(value, 0b11);
  let e = bits::complete::tag::<_, u8, _, Error<_>>(0b10, 2usize)((&i[..], 0)).unwrap_err();
  eprintln!("{e:?}");
  let ((r, offset), value) = bits::streaming::take::<_, u8, _, Error<_>>(2usize)((&i[..], 0)).unwrap();
  assert_eq!(r, i);
  assert_eq!(offset, 2);
  assert_eq!(value, 0b11u8);
  let (r, (flag, value)) = bits::bits::<_, (u8, u16), Error<_>, Error<_>, _>(nom::sequence::pair(bits::complete::tag(0b11, 2usize),bits::complete::take(14usize),))(&i[..]).unwrap();
  assert_eq!(r, b"");
  assert_eq!(flag, 0b11);
  assert_eq!(value, 0b10_0101_0110_1000);
}