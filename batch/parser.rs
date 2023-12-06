#![allow(dead_code)]
use nom::{
  bits,
  branch::alt,
  bytes::complete::{tag, take},
  character::complete::char as nom_char,
  combinator::{map, value},
  error::Error,
  multi::{count, length_data, many_till},
  number::complete::be_u8,
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
enum DomainPart<'a> {
  Pointer(usize),
  Label(&'a [u8]),
  Terminator,
}

fn parse_pointer(i: &[u8]) -> IResult<&[u8], DomainPart> {
  map(
    bits::bits::<_, _, Error<_>, _, _>(preceded(
      bits::complete::tag(0b11, 2usize),
      bits::complete::take(14usize),
    )),
    DomainPart::Pointer,
  )(i)
}

fn parse_terminator(i: &[u8]) -> IResult<&[u8], DomainPart> {
  value(DomainPart::Terminator, tag(&[0u8]))(i)
}

fn parse_label(i: &[u8]) -> IResult<&[u8], DomainPart> {
  map(
    length_data(bits::bits::<_, usize, Error<_>, _, _>(preceded(
      bits::complete::tag(0b00, 2usize),
      bits::complete::take(6usize),
    ))),
    DomainPart::Label,
  )(i)
}

pub fn parse_domain(i: &[u8]) -> IResult<&[u8], &[u8]> {
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