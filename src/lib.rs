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

// /// High level read/write trait for UDP connections to implement
// pub trait Stream {
//     /// Writes an Udp command
//     ///
//     /// # Arguments
//     ///
//     /// `command` - Command to write
//     fn write(&self, command: Vec<u8>) -> Result<()>;

//     /// Reads from UDP Stream result
//     ///
//     fn read(&self, rx_len: usize) -> Result<Vec<u8>>;

//     /// Writes Udp command and reads result
//     ///
//     /// # Arguments
//     ///
//     /// `command` - Command to write and read from
//     fn transfer(&self, command: Vec<u8>, rx_len: usize) -> Result<Vec<u8>>;
// }

/// This is the actual stream that data is tranferred over
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
impl Stream for UdpStream {
    /// Writing
    fn write(&self, command: Vec<u8>) -> Result<()> {
        self.socket.connect(self.target)?;
        match self.socket.send(&command) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }
    /// Reading
    fn read(&self, buf: &mut Vec<u8>, _rx_len: usize) -> Result<Vec<u8>> {
        self.socket.connect(self.target)?;
        // let mut buf = Vec::with_capacity(rx_len);
        match self.socket.recv(buf) {
            Ok(len) => Ok(buf[..len].to_vec()),
            Err(e) => Err(e),
        }
    }
    fn read_timeout(&self, buf: &mut Vec<u8>, _rx_len: usize, timeout: Duration) -> Result<Vec<u8>> {
        self.socket.connect(self.target)?;
        // let mut buf = Vec::with_capacity(rx_len);
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
    fn transfer(&self, command: Vec<u8>, rx_len: usize, _delay: Duration) -> Result<Vec<u8>> {
        self.socket.connect(self.target)?;
        let mut buf = Vec::with_capacity(rx_len);
        match self.socket.send(&command) {
            Ok(_) => {
                match self.socket.recv(&mut buf) {
                    Ok(len) => Ok(buf[..len].to_vec()),
                    Err(e) => Err(e),
                }
            },
            Err(e) => Err(e),
        }
    }
}

/// Struct for communicating with a device via UDP
pub struct Connection {
    stream: Box<dyn Stream>,
}
impl Connection {
    /// I2C connection constructor
    fn new(udp: Box<dyn Stream>) -> Self {
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
    pub fn transfer(&self, command: Vec<u8>, rx_len: usize) -> Result<Vec<u8>> {
        let delay = Duration::new(0,0);
        self.stream.transfer(command,rx_len,delay)
    }
}


