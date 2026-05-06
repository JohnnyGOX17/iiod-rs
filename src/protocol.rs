use crate::error::{Error, Result};
use crate::transport::Transport;

/// Send the `VERSION` command and return the (major, minor, git_tag) tuple.
pub fn version(transport: &mut dyn Transport) -> Result<(u32, u32, String)> {
    transport.send(b"VERSION\n")?;
    let line = transport.recv_line()?;

    // Response format: "<major>.<minor>.<git_tag>" e.g. "0.24.gc4498aa"
    let parts: Vec<&str> = line.splitn(3, '.').collect();
    if parts.len() < 2 {
        return Err(Error::Protocol(format!("unexpected VERSION response: {line}")));
    }

    let major = parts[0]
        .parse::<u32>()
        .map_err(|e| Error::Protocol(format!("bad major version: {e}")))?;
    let minor = parts[1]
        .parse::<u32>()
        .map_err(|e| Error::Protocol(format!("bad minor version: {e}")))?;
    let git_tag = parts.get(2).unwrap_or(&"").to_string();

    Ok((major, minor, git_tag))
}

/// Send the `PRINT` command and return the raw XML string describing all
/// devices, channels, and attributes on the remote IIO context.
pub fn print(transport: &mut dyn Transport) -> Result<String> {
    transport.send(b"PRINT\n")?;
    let line = transport.recv_line()?;

    let len: i32 = line
        .trim()
        .parse()
        .map_err(|e| Error::Protocol(format!("bad PRINT length: {e}")))?;

    if len < 0 {
        return Err(Error::IiodError {
            code: len,
            msg: errno_to_msg(len),
        });
    }

    let mut buf = vec![0u8; len as usize];
    transport.recv_exact(&mut buf)?;

    String::from_utf8(buf).map_err(|e| Error::Protocol(format!("PRINT response not UTF-8: {e}")))
}

/// Parse a response line as a return code. Positive = success/length,
/// negative = `-errno`.
pub fn parse_return_code(line: &str) -> Result<i32> {
    let code: i32 = line
        .trim()
        .parse()
        .map_err(|e| Error::Protocol(format!("expected numeric return code: {e}")))?;
    if code < 0 {
        Err(Error::IiodError {
            code,
            msg: errno_to_msg(code),
        })
    } else {
        Ok(code)
    }
}

/// Best-effort mapping of negative errno codes to human-readable strings.
fn errno_to_msg(code: i32) -> String {
    match -code {
        1 => "operation not permitted".into(),
        2 => "no such file or directory".into(),
        5 => "I/O error".into(),
        6 => "no such device or address".into(),
        12 => "out of memory".into(),
        13 => "permission denied".into(),
        16 => "device busy".into(),
        19 => "no such device".into(),
        22 => "invalid argument".into(),
        32 => "broken pipe".into(),
        110 => "connection timed out".into(),
        111 => "connection refused".into(),
        _ => format!("errno {}", -code),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::Transport;
    use std::collections::VecDeque;

    /// A mock transport that replays canned responses.
    struct MockTransport {
        lines: VecDeque<String>,
        data: Vec<u8>,
        sent: Vec<Vec<u8>>,
    }

    impl MockTransport {
        fn new(lines: Vec<&str>, data: Vec<u8>) -> Self {
            Self {
                lines: lines.into_iter().map(String::from).collect(),
                data,
                sent: Vec::new(),
            }
        }
    }

    impl Transport for MockTransport {
        fn send(&mut self, cmd: &[u8]) -> crate::error::Result<()> {
            self.sent.push(cmd.to_vec());
            Ok(())
        }

        fn recv_line(&mut self) -> crate::error::Result<String> {
            self.lines
                .pop_front()
                .ok_or_else(|| Error::Protocol("no more lines".into()))
        }

        fn recv_exact(&mut self, buf: &mut [u8]) -> crate::error::Result<()> {
            let n = buf.len();
            if self.data.len() < n {
                return Err(Error::Protocol("not enough data".into()));
            }
            buf.copy_from_slice(&self.data[..n]);
            self.data.drain(..n);
            Ok(())
        }
    }

    #[test]
    fn test_version_parse() {
        let mut t = MockTransport::new(vec!["0.24.gc4498aa"], vec![]);
        let (major, minor, tag) = version(&mut t).unwrap();
        assert_eq!(major, 0);
        assert_eq!(minor, 24);
        assert_eq!(tag, "gc4498aa");
        assert_eq!(t.sent[0], b"VERSION\n");
    }

    #[test]
    fn test_print_success() {
        let xml = "<context><device id=\"iio:device0\" name=\"ad9361-phy\"></device></context>";
        let len_line = format!("{}", xml.len());
        let mut t = MockTransport::new(vec![&len_line], xml.as_bytes().to_vec());
        let result = print(&mut t).unwrap();
        assert_eq!(result, xml);
    }

    #[test]
    fn test_print_error() {
        let mut t = MockTransport::new(vec!["-19"], vec![]);
        let err = print(&mut t).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("no such device"), "got: {msg}");
    }

    #[test]
    fn test_return_code_positive() {
        assert_eq!(parse_return_code("42").unwrap(), 42);
    }

    #[test]
    fn test_return_code_negative() {
        let err = parse_return_code("-22").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("invalid argument"), "got: {msg}");
    }
}
