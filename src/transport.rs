use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use crate::error::{Error, Result};

/// Default iiod TCP port.
pub const IIOD_PORT: u16 = 30431;

/// Abstraction over the byte transport to an `iiod` daemon.
pub trait Transport: Send {
    /// Send a raw command (caller is responsible for the trailing newline).
    fn send(&mut self, cmd: &[u8]) -> Result<()>;
    /// Read a single `\n`-terminated line (newline stripped).
    fn recv_line(&mut self) -> Result<String>;
    /// Read exactly `len` bytes into `buf`.
    fn recv_exact(&mut self, buf: &mut [u8]) -> Result<()>;
}

/// Blocking TCP transport backed by `std::net::TcpStream`.
pub struct TcpTransport {
    reader: BufReader<TcpStream>,
    writer: TcpStream,
}

impl TcpTransport {
    /// Connect to an `iiod` instance at `addr` (e.g. `"192.168.2.1"` or
    /// `"192.168.2.1:30431"`). If no port is given, the default iiod port
    /// (30431) is used.
    ///
    /// `timeout` controls the TCP connect timeout.
    pub fn connect(addr: &str, timeout: Duration) -> Result<Self> {
        let sock_addr = if addr.contains(':') {
            addr.to_socket_addrs()?
                .next()
                .ok_or_else(|| Error::Protocol(format!("cannot resolve address: {addr}")))?
        } else {
            format!("{addr}:{IIOD_PORT}")
                .to_socket_addrs()?
                .next()
                .ok_or_else(|| Error::Protocol(format!("cannot resolve address: {addr}")))?
        };

        let stream = TcpStream::connect_timeout(&sock_addr, timeout)?;
        stream.set_nodelay(true)?;

        let writer = stream.try_clone()?;
        let reader = BufReader::new(stream);

        Ok(Self { reader, writer })
    }
}

impl Transport for TcpTransport {
    fn send(&mut self, cmd: &[u8]) -> Result<()> {
        self.writer.write_all(cmd)?;
        self.writer.flush()?;
        Ok(())
    }

    fn recv_line(&mut self) -> Result<String> {
        let mut line = String::new();
        let n = self.reader.read_line(&mut line)?;
        if n == 0 {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "connection closed",
            )));
        }
        // Strip trailing newline / carriage return
        if line.ends_with('\n') {
            line.pop();
        }
        if line.ends_with('\r') {
            line.pop();
        }
        Ok(line)
    }

    fn recv_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        self.reader.read_exact(buf)?;
        Ok(())
    }
}
