pub use unisocket::SocketAddr;
use unisocket::{Stream, Listener};
use std::io;
use std::time::Duration;
use std::net::{TcpStream, Shutdown};
#[cfg(unix)]
use std::os::unix::net as unix;
use std::io::{Read, Write};
use std::fmt;
use std::fmt::{Formatter, Debug};


#[derive(Debug)]
pub enum WriteErr{
    I0(io::Error),
    TooLongFrame,
}

impl fmt::Display for WriteErr{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self{
            WriteErr::I0(err) => { std::fmt::Display::fmt(&err, f) }
            WriteErr::TooLongFrame => {
                write!(f, "{}", "Frame is too long to send by SFP")
            }
        }
    }
}

#[derive(Debug)]
pub struct Connection{
    pub stream: Stream
}

impl From<Stream> for Connection{
    fn from(stream: Stream) -> Self {
        Self{stream}
    }
}

impl From<TcpStream> for Connection{
    fn from(s: TcpStream) -> Self {
        Self::from(Stream::from(s))
    }
}

#[cfg(unix)]
impl From<unix::UnixStream> for Connection{
    fn from(s: unix::UnixStream) -> Self {
        Self::from(Stream::from(s))
    }
}

impl Connection{
    pub fn connect(s: &SocketAddr) -> io::Result<Self> {
        match Stream::connect(s) {
            Ok(stream) => {
                Ok(Self::from(stream))
            }
            Err(err) => {
                Err(err)
            }
        }
    }
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.stream.local_addr()
    }
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.stream.peer_addr()
    }
    pub fn set_read_timeout(&self, t: Option<Duration>) -> io::Result<()> {
        self.stream.set_read_timeout(t)
    }
    pub fn set_write_timeout(&self, t: Option<Duration>) -> io::Result<()> {
        self.stream.set_write_timeout(t)
    }
    pub fn shutdown(&self, t: Shutdown) -> io::Result<()> {
        self.stream.shutdown(t)
    }
    pub fn try_clone(&self) -> io::Result<Self>{
        Ok(Self::from(self.stream.try_clone()?))
    }
    pub fn read_frame(&mut self) -> io::Result<Vec<u8>>{
        let mut header = [0u8; 4];
        self.stream.read_exact(&mut header)?;
        let length = u32::from_be_bytes(header) as usize;
        let mut frame = vec![0u8; length];
        self.stream.read_exact(&mut *frame)?;
        Ok(frame)
    }
    pub fn write_frame(&mut self, frame: &mut [u8]) -> Result<(), WriteErr>{
        let length = frame.len();
        if length > u32::MAX as usize {
            return Err(WriteErr::TooLongFrame)
        }
        let mut header:[u8; 4] = u32::to_be_bytes(length as u32);
        if let Err(err) = self.stream.write_all(&mut header){return Err(WriteErr::I0(err))}
        if let Err(err) = self.stream.write_all(frame){return Err(WriteErr::I0(err))}
        Ok(())
    }
    pub fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

impl Iterator for Connection{
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.read_frame(){
            Ok(frame) => {Some(frame)}
            Err(_) => {None}
        }
    }
}

pub struct Server{
    listener: Listener
}

impl From<Listener> for Server{
    fn from(listener: Listener) -> Self {
        Self{listener}
    }
}

impl Server{
    pub fn bind(s: &SocketAddr) -> io::Result<Self> {
        Ok(Self{listener: Listener::bind(s)?})
    }
    pub fn bind_reuse(s: &SocketAddr, _mode: Option<u32>) -> io::Result<Self> {
        Ok(Self{listener: Listener::bind_reuse(s, _mode)?})
    }
    pub fn accept(&self) -> io::Result<(Connection,SocketAddr)> {
        let (stream, addr) = self.listener.accept()?;
        Ok((Connection::from(stream), addr))
    }
}

impl Iterator for Server{
    type Item = (Connection,SocketAddr);

    fn next(&mut self) -> Option<Self::Item> {
        match self.accept(){
            Ok((conn, addr)) => {
                Some((conn, addr))
            }
            Err(_) => { None }
        }
    }
}
