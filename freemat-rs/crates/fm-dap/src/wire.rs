//! DAP wire framing: each message is a JSON object prefixed with an
//! HTTP-style `Content-Length` header, exactly like the Language Server
//! Protocol:
//!
//! ```text
//! Content-Length: 119\r\n
//! \r\n
//! { "seq": 1, "type": "request", ... }
//! ```
//!
//! These helpers are transport-agnostic — they work over any [`BufRead`] /
//! [`Write`], so the server runs identically over stdio (real editors) and over
//! a TCP loopback (the integration tests).

use std::io::{self, BufRead, Write};

use serde_json::Value;

/// Read one framed DAP message. Returns `Ok(None)` at a clean end-of-stream
/// (the peer closed the connection), or an error on a malformed frame.
pub fn read_message<R: BufRead>(reader: &mut R) -> io::Result<Option<Value>> {
    let mut content_length: Option<usize> = None;

    // Header block: `Key: Value` lines terminated by a blank line.
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            // EOF. If it lands mid-frame that's an error; at a boundary it's a
            // clean disconnect.
            return if content_length.is_some() {
                Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "EOF in the middle of a DAP message header",
                ))
            } else {
                Ok(None)
            };
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break; // end of headers
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_length = Some(value.trim().parse().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "bad Content-Length value")
            })?);
        }
        // Other headers (e.g. Content-Type) are ignored.
    }

    let len = content_length
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing Content-Length"))?;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;
    let value =
        serde_json::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(Some(value))
}

/// Write one framed DAP message.
pub fn write_message<W: Write>(writer: &mut W, msg: &Value) -> io::Result<()> {
    let body = serde_json::to_vec(msg)?;
    write!(writer, "Content-Length: {}\r\n\r\n", body.len())?;
    writer.write_all(&body)?;
    writer.flush()
}
