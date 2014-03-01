#[feature(macro_rules)];

use std::str;
use std::io;
use std::io::{Writer, IoResult, Reader, InvalidInput};
use std::result::{Err, Ok};

macro_rules! fail_if_err(
  ($inp:expr) => (
    match $inp {
      Err(res) => fail!(format!("error: {}", res)),
      _ => {}
    }
  );
)

macro_rules! unwrap_return_err(
  ($inp:expr) => (
    match $inp {
      Err(res) => return Err(res),
      Ok(a) => a
    }
  );
)


pub trait OscType : Send {
  fn write_to(&self, out: &mut Writer) -> IoResult<()>;
  fn type_tag(&self) -> char;
  fn from_reader(reader: &mut Reader) -> IoResult<Self>;
}

impl OscType for ~str {
  fn write_to(&self, out: &mut Writer) -> IoResult<()> {
    let mut len = self.len() + 1;
    let instr_len = self.len();
    if len & 3 != 0 {
      // round up to nearest multiple of 4
      len = (len|3) + 1;
    }

    for b in self.bytes() {
      unwrap_return_err!(out.write_u8(b));
    }

    for _ in range(instr_len, len) {
      unwrap_return_err!(out.write_u8(0 as u8));
    }

    Ok(())
  }

  fn type_tag(&self) -> char {
    return 's';
  }

  fn from_reader(reader: &mut Reader) -> IoResult<~str> {
    let mut str_bytes : ~[u8] = ~[];

    loop {
      match reader.read_byte() {
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
      match reader.read_byte() {
        Err(err) => return Err(err),
        _ => {}
      }
    }

    return match str::from_utf8_owned(str_bytes) {
      Some(val) => Ok(val),
      None() => Err(io::standard_error(InvalidInput))
    };
  }
}

impl OscType for ~[u8] {
  fn write_to(&self, out: &mut Writer) -> IoResult<()> {
    let size = self.len();
    unwrap_return_err!(out.write_be_i32(size as i32));
    return out.write(*self)
  }

  fn type_tag(&self) -> char {
    return 'b';
  }

  fn from_reader(reader: &mut Reader) -> IoResult<~[u8]> {
    let len = match reader.read_be_i32() {
      Ok(len) => len,
      Err(err) => return Err(err)
    };

    return reader.read_bytes(len as uint);
  }
}

impl OscType for i32 {
  fn write_to(&self, out: &mut Writer) -> IoResult<()> {
    return out.write_be_i32(*self);
  }

  fn type_tag(&self) -> char {
    return 'i';
  }

  fn from_reader(reader: &mut Reader) -> IoResult<i32> {
    return reader.read_be_i32();
  }
}

impl OscType for f32 {
  fn write_to(&self, out: &mut Writer) -> IoResult<()> {
    return out.write_be_f32(*self);
  }

  fn type_tag(&self) -> char {
    return 'f';
  }

  fn from_reader(reader: &mut Reader) -> IoResult<f32> {
    return reader.read_be_f32();
  }
}



pub struct OscMessage {
  // structure: addr pattern string tt string, zero or more arguments
  address: ~str,
  arguments: ~[~OscType]
}

fn make_osc_result<T: OscType>(result: IoResult<T>) -> IoResult<~OscType> {
  match result {
    Err(err) => return Err(err),
    Ok(val) => return Ok(~val as ~OscType)
  }
}

fn osctype_from_typetag_and_reader(typetag: u8, reader: &mut Reader) -> IoResult<~OscType> {
  match typetag as char {
    'f' => {
      let ret: IoResult<f32> = OscType::from_reader(reader);
      return make_osc_result(ret);
    },
    'i' => {
      let ret: IoResult<i32> = OscType::from_reader(reader);
      return make_osc_result(ret);
    },
    's' => {
      let ret: IoResult<~str> = OscType::from_reader(reader);
      return make_osc_result(ret);
    },
    'b' => {
      let ret: IoResult<~[u8]> = OscType::from_reader(reader);
      return make_osc_result(ret);
    }

    t => {
      fail!(format!("No implementation of typetag: {}", t));
    }
  }

}

impl OscMessage {
  pub fn write_to(&self, out: &mut Writer) -> IoResult<()> {
    unwrap_return_err!(self.address.write_to(out));
    // typetag string is ",[isfb]*"
    let mut typetags = ~[',' as u8];
    for arg in self.arguments.iter() {
      typetags.push(arg.type_tag() as u8);
    }

    let typetags_str = str::from_utf8_owned(typetags);
    match typetags_str {
      Some(tt) => unwrap_return_err!(tt.write_to(out)),
      None() => return Err(io::standard_error(InvalidInput))
    }

    for arg in self.arguments.iter() {
      unwrap_return_err!(arg.write_to(out));
    }

    Ok(())
  }

  pub fn from_reader(reader: &mut Reader) -> IoResult<~OscMessage> {
    let address: ~str = unwrap_return_err!(OscType::from_reader(reader));
    let typetags: ~str = unwrap_return_err!(OscType::from_reader(reader));
    println!("addr: {}", address);
    println!("tt: {}", typetags);
    let arguments: ~[~OscType] = typetags.slice_from(1).bytes().map(|typetag| match osctype_from_typetag_and_reader(typetag, reader) {
      Ok(ot) => ot,
      Err(err) => fail!("panicked on error: {}", err)
    }).to_owned_vec();
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
    fail_if_err!((~"asdf").write_to(&mut writer));

    assert_eq!(writer.unwrap(), expected);
  }

  #[test]
  fn test_read_osc_string() {
    let data = (~"asdf\0\0\0\0").into_bytes();
    let mut reader = MemReader::new(data);
    let actual: IoResult<~str> = OscType::from_reader(&mut reader);
    match actual {
      Ok(val) => assert_eq!(val, ~"asdf"),
      e => fail!("error: {}", e)
    }
  }


  #[test]
  fn test_write_osc_blob() {
    let expected = ~[0u8, 0u8, 0u8, 5u8, 1u8, 2u8, 3u8, 4u8, 5u8];
    let mut writer = MemWriter::new();
    fail_if_err!((~[1u8, 2u8, 3u8, 4u8, 5u8]).write_to(&mut writer));

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
    fail_if_err!((0x00112233).write_to(&mut writer));

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
    fail_if_err!((1.234).write_to(&mut writer));

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

  #[test]
  fn test_write_osc_message() {
    let expected = (~"/test/do\0\0\0\0,ss\0Hello\0\0\0world\0\0\0").into_bytes();
    let mut writer = MemWriter::new();
    let msg = OscMessage { address: ~"/test/do", arguments: ~[~~"Hello" as ~OscType, ~~"world" as ~OscType] };
    fail_if_err!(msg.write_to(&mut writer));
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
