//
// Copyright (C) 2022 CUAVA
//
// Licensed under the Apache License, Version 2.0 (the "License")
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

#![deny(missing_docs)]

//! A generalized HAL for communicating over UDP

use std::net::{UdpSocket,SocketAddr};
use std::str::FromStr;
use std::io::Result;
use hal_stream::Stream;
use std::time::Duration;
use math::round::ceil;

/// Max size for UDP packages
pub const MAX_BUFFER_SIZE: usize = 256;

/// Trait for UDP message handling with unlimited size
pub trait Message {
    /// Receive function
    fn recv_msg(&self) -> Result<(Vec<u8>,SocketAddr)>;
    /// Send function
    fn send_msg(&self, msg: &Vec<u8>, to: &SocketAddr) -> Result<()>;
}

impl Message for UdpSocket {
    fn recv_msg(&self) -> Result<(Vec<u8>,SocketAddr)> {
        let mut buf = [0u8; MAX_BUFFER_SIZE];
        let mut out: Vec<u8> = Vec::new();
        loop {
            match self.recv_from(&mut buf) {
                Ok((b,a)) => {
                    out.append(&mut buf[1..b].to_vec());
                    if buf[0] > 0 {
                        #[cfg(test)]
                        println!("{:?}",buf[0]);
                        continue;
                    }                    
                    else {
                        return Ok((out,a));
                    }
                }
                Err(e) => return Err(e),
            }
        }
    }

    fn send_msg(&self, msg: &Vec<u8>, to: &SocketAddr) -> Result<()> {
        // let mut buf = [0u8; MAX_BUFFER_SIZE];
        let mut buf = msg.clone();
        loop {
            if buf.len() > MAX_BUFFER_SIZE-1 {
                #[cfg(test)]
                println!("{:?}",ceil(((buf.len()-1)/(MAX_BUFFER_SIZE-1)) as f64,0) as u8);
                buf.insert(0,(ceil(((buf.len()-1)/(MAX_BUFFER_SIZE-1)) as f64,0)) as u8);
                self.send_to(buf.drain(..MAX_BUFFER_SIZE).as_slice(), to)?;
            }
            else {
                buf.insert(0,0);
                match self.send_to(&buf,to) {
                    Ok(_) => return Ok(()),
                    Err(e) => return Err(e),
                }
            }
        }
    }
}

/// This is the actual stream that data is tranferred over
#[derive(Debug)]
pub struct UdpStream {
    socket: UdpSocket,
    target: SocketAddr,
}
impl UdpStream {
    /// create new Stream Instance
    /// 
    /// # Arguments
    /// 
    /// socket: String IP-Address and Socket to bind
    /// target: String IP-Address and Socket to connect to for sending and receiving data
    pub fn new(socket: String, target: String) -> Self {
        Self {
            socket: UdpSocket::bind(socket).unwrap(),
            target: SocketAddr::from_str(&target).unwrap(),
        }
    }
}
/// Implementation of Trait Stream for Stream Instance
impl hal_stream::Stream for UdpStream {
    type StreamError = std::io::Error;
    /// Writing
    fn write(&self, command: Vec<u8>) -> Result<()> {
        self.socket.connect(self.target)?;
        match self.socket.send(&command) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }
    /// Writing single bytes in multiple write commands
    fn write_bytes(&self, command: Vec<u8>) -> Result<()> {
        self.socket.connect(self.target)?;
        for d in command {
            match self.socket.send(&[d]) {
                Ok(_) => continue,
                Err(e) => {
                    return Err(e);
                },
            }
        }
        Ok(())
    }
    /// Reading
    fn read(&self, buf: &mut Vec<u8>, _rx_len: usize) -> Result<Vec<u8>> {
        self.socket.connect(self.target)?;
        match self.socket.recv(buf) {
            Ok(len) => Ok(buf[..len].to_vec()),
            Err(e) => Err(e),
        }
    }
    /// Reading with timeout
    fn read_timeout(&self, buf: &mut Vec<u8>, _rx_len: usize, timeout: Duration) -> Result<Vec<u8>> {
        self.socket.connect(self.target)?;
        self.socket.set_read_timeout(Some(timeout));
        match self.socket.recv(buf) {
            Ok(len) => {
                self.socket.set_read_timeout(None);
                Ok(buf[..len].to_vec())
            }
            Err(e) => Err(e),
        }
    }
    /// Write/Read transfer
    fn transfer(&self, command: Vec<u8>, _rx_len: usize, delay: Option<Duration>) -> Result<Vec<u8>> {
        self.socket.set_read_timeout(delay);
        match self.socket.send_msg(&command,&self.target) {
            Ok(()) => match self.socket.recv_msg() {
                Ok((msg,from)) => Ok(msg),
                Err(e) => Err(e),
            }
            Err(e) => Err(e),
        }
    }
}

/// Struct for communicating with a device via UDP
pub struct Connection {
    stream: Box<dyn Stream<StreamError = std::io::Error> + Send>,
}
impl Connection {
    /// I2C connection constructor
    fn new(udp: Box<dyn Stream<StreamError = std::io::Error> + Send>) -> Self {
        Self {
            stream: udp
        }
    }

    /// Convenience constructor to create connection from bus path
    ///
    /// # Arguments
    ///
    /// `path` - Path to I2C device
    /// `slave` - I2C slave address to read/write to
    pub fn from_path(socket: String, target: String) -> Self {
        Self {
            stream: Box::new(UdpStream::new(socket,target)),
        }
    }

    /// Writes an Udp command
    ///
    /// # Arguments
    ///
    /// `command` - Command to write
    pub fn write(&self, command: Vec<u8>) -> Result<()> {
        self.stream.write(command)
    }

    /// Reads from UDP Stream result
    ///
    pub fn read(&self, rx_len: usize) -> Result<Vec<u8>> {
        let mut buf = Vec::with_capacity(rx_len);
        self.stream.read(&mut buf,rx_len)
    }

    /// Writes Udp command and reads result
    ///
    /// # Arguments
    ///
    /// `command` - Command to write and read from
    pub fn transfer(&self, command: Vec<u8>) -> Result<Vec<u8>> {
        // let delay = Duration::new(0,0);
        let rx_len = 0;
        self.stream.transfer(command,rx_len,None)
    }

    /// Writes UDP command and reads result, read times out after timeout
    /// # Arguments
    /// `command` - Command to write and read from
    /// `timeout` - Timeout for the read operation
    pub fn transfer_timeout(&self, command: Vec<u8>, timeout: Duration) -> Result<Vec<u8>> {
        let rx_len = 0;
        self.stream.transfer(command,rx_len,Some(timeout))
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr,Ipv6Addr};
    use std::str::FromStr;

    #[test]
    fn test_recv_msg() {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let addr = socket.local_addr().unwrap();
        let msg = vec![1,2,3,4,5,6,7,8,9,10];
        let mut buf = [0u8; 11];
        buf[0] = 0;
        buf[1..11].copy_from_slice(&msg);
        socket.send_to(&buf,&addr).unwrap();
        let (recv_msg,_) = socket.recv_msg().unwrap();
        assert_eq!(msg,recv_msg);
    }

    #[test]
    fn test_send_msg() {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let addr = socket.local_addr().unwrap();
        let msg = vec![1,2,3,4,5,6,7,8,9,10];
        socket.send_msg(&msg,&addr).unwrap();
        let mut buf = [0u8; MAX_BUFFER_SIZE];
        let (b,_) = socket.recv_from(&mut buf).unwrap();
        assert_eq!(msg,buf[1..b].to_vec());
    }

    #[test]
    fn test_send_recv_msg() {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let addr = socket.local_addr().unwrap();
        let mut msg = [0u8; 1021].to_vec();
        for i in 0..msg.len() {
            msg[i] = i as u8;
        }
        socket.send_msg(&msg,&addr).unwrap();
        let (recv_msg,_) = socket.recv_msg().unwrap();
        assert_eq!(msg,recv_msg);
    }
}
