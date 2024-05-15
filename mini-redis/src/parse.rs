use std::{vec, str};
use std::fmt::{Debug, Display, Formatter};
use bytes::Bytes;
use crate::Frame;

#[derive(Debug)]
pub(crate) struct Parse {
    parts: vec::IntoIter<Frame>,
}

#[derive(Debug)]
pub(crate) enum ParseError {
    EndOfStream,
    Other(crate::Error),
}

impl Parse {
    pub(crate) fn new(frame: Frame) -> Result<Parse, ParseError> {
        let array = match frame {
            Frame::Array(array) => array,
            frame => return Err(format!("parse error, expect array, got{:?}", frame).into()),
        };
        Ok(Parse{
            parts: array.into_iter(),
        })
    }

    fn next(&mut self) -> Result<Frame, ParseError> {
        self.parts.next().ok_or(ParseError::EndOfStream)
    }

    pub(crate) fn next_string(&mut self) -> Result<String, ParseError> {
        match self.next()? {
            Frame::Simple(s) => Ok(s),
            Frame::Bulk(data) => {
                str::from_utf8(&data[..])
                    .map(|s| s.to_string())
                    .map_err(|_| "invalid string".into())
            }
            frame => Err(format!("next_string error, expect simple or bulk, got {:?}", frame).into())
        }
    }

    pub(crate) fn next_bytes(&mut self) -> Result<Bytes, ParseError> {
        match self.next()? {
            Frame::Simple(s) => Ok(Bytes::from(s.into_bytes())),
            Frame::Bulk(data) => Ok(data),
            frame => Err(format!("next_bytes error, expect simple or bulk, got {:?}", frame).into())
        }
    }

    pub(crate) fn next_int(&mut self) -> Result<u64, ParseError> {
        use atoi::atoi;
        const MSG: &str = "invalid number";
        match self.next()? {
            Frame::Integer(v) => Ok(v),
            Frame::Simple(data) => atoi::<u64>(data.as_bytes()).ok_or_else(||MSG.into()),
            Frame::Bulk(data) => atoi::<u64>(&data).ok_or_else(||MSG.into()),
            frame => Err(format!("next_int error, expect int, got {:?}", frame).into())
        }
    }

    pub(crate) fn finish(&mut self) -> Result<(), ParseError> {
        if self.parts.next().is_none() {
            Ok(())
        } else {
            Err("expect end of frame, but more".into())
        }
    }
}

impl From<String> for ParseError {
    fn from(src: String) -> Self {
        ParseError::Other(src.into())
    }
}

impl From<&str> for ParseError {
    fn from(src: &str) -> Self {
        ParseError::Other(src.to_string().into())
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::EndOfStream => std::fmt::Display::fmt("unexpected end of stream", f),
            ParseError::Other(err) => std::fmt::Display::fmt(&err, f),
        }
    }
}

impl std::error::Error for ParseError {}