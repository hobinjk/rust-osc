#[feature(macro_rules)];

use osc::{OscType, OscMessage, OscString, OscInt, OscFloat, OscBlob};
use std::io::net::ip::{Ipv4Addr, SocketAddr};
use std::io::net::udp::{UdpSocket};
use std::io::timer;
use std::os;

mod osc;

fn print_usage(name: ~str) {
  println!("usage: ./{} <listenPort> <sendPort>", name);
}

fn main() {
  let args = os::args();
  match args.len() {
    3 => {},
    _ => {
      print_usage(args[0]);
      return;
    }
  };

  let localPort = match from_str::<int>(args[1]) {
    Some(port) => port as u16,
    None() => {
      print_usage(args[0]);
      return;
    }
  };

  let remotePort = match from_str::<int>(args[2]) {
    Some(port) => port as u16,
    None() => {
      print_usage(args[0]);
      return;
    }
  };


  let localAddr = SocketAddr {ip: Ipv4Addr(127, 0, 0, 1), port: localPort};
  let remoteAddr = SocketAddr {ip: Ipv4Addr(127, 0, 0, 1), port: remotePort};

  let udpSocket = UdpSocket::bind(localAddr).unwrap();
  let readSocket = udpSocket.clone();
  let writeSocket = udpSocket.clone();

  spawn(proc() {
    let mut udpStream = readSocket.connect(remoteAddr);
    loop {
      let msg = OscMessage::from_reader(&mut udpStream).unwrap();
      println!("recv {}: {:?}", msg.address, msg.arguments);
    }
  });

  spawn(proc() {
    let mut udpStream = writeSocket.connect(remoteAddr);
    loop {
      let msg = OscMessage { address: ~"/test", arguments: ~[OscString(~"Hello"), OscInt(4)] };
      msg.write_to(&mut udpStream).unwrap();
      println!("send {}: {:?}", msg.address, msg.arguments);
      timer::sleep(500);
    }
  });
}
