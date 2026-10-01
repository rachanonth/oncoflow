use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, Read, Write};

pub(super) const SERVER_NAME: &str = "oncoflow.local";
pub(super) const PROTOCOL_VERSION: u32 = 1;
const MAX_FRAME: usize = 8 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
pub(super) struct Request {
    pub version: u32,
    pub command: String,
    pub args: Value,
}

#[derive(Serialize, Deserialize)]
pub(super) struct Response {
    pub result: Result<Value, Value>,
}

pub(super) fn read_frame<T: DeserializeOwned>(stream: &mut impl Read) -> io::Result<T> {
    let mut length = [0; 4];
    stream.read_exact(&mut length)?;
    let length = u32::from_be_bytes(length) as usize;
    if length == 0 || length > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid message length",
        ));
    }
    let mut data = vec![0; length];
    stream.read_exact(&mut data)?;
    serde_json::from_slice(&data)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid message"))
}

pub(super) fn write_frame(stream: &mut impl Write, value: &impl Serialize) -> io::Result<()> {
    let data = serde_json::to_vec(value).map_err(io::Error::other)?;
    if data.len() > MAX_FRAME {
        return Err(io::Error::other("Message too large"));
    }
    stream.write_all(&(data.len() as u32).to_be_bytes())?;
    stream.write_all(&data)?;
    stream.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unbounded_frames_before_allocation() {
        assert!(read_frame::<Value>(&mut &u32::MAX.to_be_bytes()[..]).is_err());
        assert!(read_frame::<Value>(&mut &0_u32.to_be_bytes()[..]).is_err());
    }
    #[test]
    fn frames_roundtrip_and_reject_truncation() {
        let mut data = Vec::new();
        write_frame(&mut data, &serde_json::json!({"command":"health_status"})).unwrap();
        assert_eq!(
            read_frame::<Value>(&mut &data[..]).unwrap()["command"],
            "health_status"
        );
        assert!(read_frame::<Value>(&mut &data[..data.len() - 1]).is_err());
    }
}
