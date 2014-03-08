#[feature(macro_rules)];

use std::str;
use std::io;
use std::vec;
use std::io::{Writer, IoResult, Reader, InvalidInput};
use std::result::{Err, Ok};

macro_rules! unwrap_return_err(
  ($inp:expr) => (
    match $inp {
      Err(res) => return Err(res),
      Ok(a) => a
    }
  );
)

pub enum OscType {
  OscString(~str),
  OscInt(i32),
  OscFloat(f32),
  OscBlob(~[u8])
}

impl OscType {
  #[inline]
  pub fn unwrap_string(self) -> ~str {
    match self {
      OscString(val) => val,
      _ => fail!("attempt to unwrap a not string as a string")
    }
  }

  #[inline]
  pub fn unwrap_blob(self) -> ~[u8] {
    match self {
      OscBlob(val) => val,
      _ => fail!("attempt to unwrap a not blob as a blob")
    }
  }

  #[inline]
  pub fn unwrap_int(self) -> i32 {
    match self {
      OscInt(val) => val,
      _ => fail!("attempt to unwrap a not string as a string")
    }
  }

  #[inline]
  pub fn unwrap_float(self) -> f32 {
    match self {
      OscFloat(val) => val,
      _ => fail!("attempt to unwrap a not float as a float")
    }
  }
}
// using traits to define methods on base traits
// using enums to denote types instead of implementing the types
// themselves
//
// Approaches:
//  writer.write_osc(stuff):
//    idiomatic-ish, short, uses type detection
//    weird to implement apparently
//
//  stuff.write_to(writer):
//    icky but easy to implement
//  writer.write(stuff.into_bytes()):
//    no, memory waste everywhere
//  OscWriter::new(writer).write(stuff):
//    type checking, possible ickiness

pub struct OscWriter<'a> {
  priv wr: &'a mut Writer
}

impl <'a> OscWriter<'a> {
  fn new<'a>(wr: &'a mut Writer) -> OscWriter<'a> {
    OscWriter {wr: wr}
  }

  pub fn write(&mut self, osc: &OscType) -> IoResult<()> {
    match osc {
      &OscString(ref val) => self.write_osc_string(val),
      &OscInt(val) => self.write_osc_int(val),
      &OscFloat(val) => self.write_osc_float(val),
      &OscBlob(ref val) => self.write_osc_blob(val)
    }
  }


  fn write_osc_string(&mut self, oscStr: &~str) -> IoResult<()> {
    let mut len = oscStr.len() + 1;
    let instr_len = oscStr.len();
    if len & 3 != 0 {
      // round up to nearest multiple of 4
      len = (len|3) + 1;
    }

    for b in oscStr.bytes() {
      unwrap_return_err!(self.wr.write_u8(b));
    }

    for _ in range(instr_len, len) {
      unwrap_return_err!(self.wr.write_u8(0 as u8));
    }

    Ok(())
  }

  fn write_osc_blob(&mut self, blob: &~[u8]) -> IoResult<()> {
    let size = blob.len();
    match self.wr.write_be_i32(size as i32) {
      Ok(_) => {},
      e => return e
    }
    self.wr.write(*blob)
  }

  fn write_osc_int(&mut self, oscInt: i32) -> IoResult<()> {
    return self.wr.write_be_i32(oscInt);
  }

  fn write_osc_float(&mut self, oscFloat: f32) -> IoResult<()> {
    return self.wr.write_be_f32(oscFloat);
  }
}

pub struct OscReader<'a> {
  priv re: &'a mut Reader
}

impl<'a> OscReader<'a> {
  pub fn new<'a>(reader: &'a mut Reader) -> OscReader<'a> {
    OscReader {re: reader}
  }

  pub fn read(&mut self, typetag: char) -> IoResult<OscType> {
    match typetag {
      's' => self.read_osc_string(),
      'i' => self.read_osc_int(),
      'f' => self.read_osc_float(),
      'b' => self.read_osc_blob(),
      _   => Err(io::standard_error(InvalidInput))
    }
  }

  fn read_osc_string(&mut self) -> IoResult<OscType> {
    let mut str_bytes : ~[u8] = ~[];

    loop {
      match self.re.read_byte() {
        Ok(0u8) => {
          break;
        },
        Ok(b) => {
          str_bytes.push(b);
        },
        Err(err) => return Err(err)
      }
    }

    let mut len = str_bytes.len() + 1;
    if len & 3 != 0 {
      len = (len|3) + 1;
    }

    for _ in range(str_bytes.len()+1, len) {
      match self.re.read_byte() {
        Err(err) => return Err(err),
        _ => {}
      }
    }

    return match str::from_utf8_owned(str_bytes) {
      Some(val) => Ok(OscString(val)),
      None() => Err(io::standard_error(InvalidInput))
    };
  }


  fn read_osc_blob(&mut self) -> IoResult<OscType> {
    let len = match self.re.read_be_i32() {
      Ok(len) => len,
      Err(err) => return Err(err)
    };

    return match self.re.read_bytes(len as uint) {
      Ok(bytes) => Ok(OscBlob(bytes)),
      Err(err) => Err(err)
    };
  }

  fn read_osc_int(&mut self) -> IoResult<OscType> {
    match self.re.read_be_i32() {
      Ok(val) => Ok(OscInt(val)),
      Err(err) => Err(err)
    }
  }

  fn read_osc_float(&mut self) -> IoResult<OscType> {
    match self.re.read_be_f32() {
      Ok(val) => Ok(OscFloat(val)),
      Err(err) => Err(err)
    }
  }
}

// drawback, no specific method accesses
pub fn get_type_tag(osc: &OscType) -> u8 {
  return match osc {
    &OscString(_) => 's' as u8,
    &OscInt(_)    => 'i' as u8,
    &OscFloat(_)  => 'f' as u8,
    &OscBlob(_)   => 'b' as u8
  };
}

pub struct OscMessage {
  // structure: addr pattern string tt string, zero or more arguments
  address: ~str,
  arguments: ~[OscType]
}

impl OscMessage {
  pub fn write_to(&self, outWriter: &mut Writer) -> IoResult<()> {
    let mut out = OscWriter::new(outWriter);
    unwrap_return_err!(out.write(&OscString(self.address.clone())));
    // typetag string is ",[isfb]*"
    let mut typetags = ~[',' as u8];
    for arg in self.arguments.iter() {
      typetags.push(get_type_tag(arg) as u8);
    }

    let typetags_str = str::from_utf8_owned(typetags);
    match typetags_str {
      Some(tt) => unwrap_return_err!(out.write(&OscString(tt))),
      None() => return Err(io::standard_error(InvalidInput))
    }

    for arg in self.arguments.iter() {
      unwrap_return_err!(out.write(arg));
    }

    Ok(())
  }

  pub fn from_reader(re: &mut Reader) -> IoResult<~OscMessage> {
    let mut reader = OscReader::new(re);
    let address = match reader.read('s') {
      Err(err) => return Err(err),
      Ok(val) => match val {
        OscString(valStr) => valStr,
        _ => ~""
      }
    };

    let typetags = match reader.read('s') {
      Err(err) => return Err(err),
      Ok(val) => match val {
        OscString(valStr) => valStr,
        _ => ~""
      }
    };

    println!("addr: {}", address);
    println!("tt: {}", typetags);
    let mut arguments: ~[OscType] = vec::with_capacity(typetags.len()-1);
    for typetag in typetags.slice_from(1).bytes() {
      arguments.push(match reader.read(typetag as char) {
        Ok(ot) => ot,
        Err(err) => fail!("panicked on error: {}", err)
      });
    }
    return Ok(~OscMessage { address: address, arguments: arguments });
  }
}

#[cfg(test)]
mod test {
  use std::io::{MemWriter, IoResult, MemReader};
  use super::{OscType, OscMessage};

  #[test]
  fn test_write_osc_string() {
    let expected = (~"asdf\0\0\0\0").into_bytes();
    let mut writer = MemWriter::new();
    let mut oscWriter = OscWriter::new(writer);
    oscWriter.write(&OscString(~"asdf")).unwrap(); // fail if err returned

    assert_eq!(writer.unwrap(), expected);
  }

  #[test]
  fn test_read_osc_string() {
    let data = (~"asdf\0\0\0\0").into_bytes();
    let mut reader = MemReader::new(data);
    let actual: IoResult<OscType> = reader.read_osc('s');
    match actual {
      Ok(OscString(val)) => assert_eq!(val, ~"asdf"),
      e => fail!("error: {}", e)
    }
  }

/*
  #[test]
  fn test_write_osc_blob() {
    let expected = ~[0u8, 0u8, 0u8, 5u8, 1u8, 2u8, 3u8, 4u8, 5u8];
    let mut writer = MemWriter::new();
    (~[1u8, 2u8, 3u8, 4u8, 5u8]).write_to(&mut writer).unwrap(); // fail if err

    assert_eq!(writer.unwrap(), expected);
  }

  #[test]
  fn test_read_osc_blob() {
    let data = ~[0u8, 0u8, 0u8, 5u8, 1u8, 2u8, 3u8, 4u8, 5u8];
    let mut reader = MemReader::new(data);
    let actual: IoResult<~[u8]> = OscType::from_reader(&mut reader);
    match actual {
      Ok(val) => assert_eq!(val, ~[1u8, 2u8, 3u8, 4u8, 5u8]),
      e => fail!("error: {}", e)
    }
  }

  #[test]
  fn test_write_osc_i32() {
    let expected = ~[00u8, 0x11u8, 0x22u8, 0x33u8];
    let mut writer = MemWriter::new();
    (0x00112233).write_to(&mut writer).unwrap(); // fail if err

    assert_eq!(writer.unwrap(), expected);
  }


  #[test]
  fn test_read_osc_i32() {
    let data = ~[00u8, 0x11u8, 0x22u8, 0x33u8];
    let mut reader = MemReader::new(data);
    let actual: IoResult<i32> = OscType::from_reader(&mut reader);
    match actual {
      Ok(val) => assert_eq!(val, 0x00112233),
      e => fail!("error: {}", e)
    }
  }

  #[test]
  fn test_write_osc_f32() {
    let expected = ~[63u8, 157u8, 243u8, 182u8];
    let mut writer = MemWriter::new();
    (1.234).write_to(&mut writer).unwrap(); // fail if err

    assert_eq!(writer.unwrap(), expected);
  }

  #[test]
  fn test_read_osc_f32() {
    let data = ~[63u8, 157u8, 243u8, 182u8];
    let mut reader = MemReader::new(data);
    let actual: IoResult<f32> = OscType::from_reader(&mut reader);
    match actual {
      Ok(val) => assert_eq!(val, 1.234),
      e => fail!("error: {}", e)
    }
  }
*/
  #[test]
  fn test_write_osc_message() {
    let expected = (~"/test/do\0\0\0\0,ss\0Hello\0\0\0world\0\0\0").into_bytes();
    let mut writer = MemWriter::new();
    let msg = OscMessage { address: ~"/test/do", arguments: ~[OscString("Hello"), OscString("world")] };
    msg.write_to(&mut writer).unwrap(); // fail if err
    assert_eq!(writer.unwrap(), expected);
  }

  #[test]
  fn test_read_osc_message() {
    let data = (~"/test/do\0\0\0\0,ss\0Hello\0\0\0world\0\0\0").into_bytes();
    let mut reader = MemReader::new(data);
    let actual: IoResult<~OscMessage> = OscMessage::from_reader(&mut reader);
    match actual {
      Ok(msg) => {
        assert_eq!(msg.address, ~"/test/do");
        assert_eq!(msg.arguments.len(), 2);
        println!("what0: {:?}", msg.arguments[0]);
        println!("what1: {:?}", msg.arguments[1]);
      },
      e => fail!("error: {:?}", e)
    }
  }
}
