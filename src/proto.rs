//! Request/response framing on the `a61ae408` / `786af345` characteristics. See PROTOCOL.md sections 2-3.

use serde::Deserialize;
use std::collections::BTreeMap;

/// Constant token seen in every message in both directions (PROTOCOL.md 2.1).
pub const TOKEN: [u8; 16] = [0x96, 0xc2, 0xf7, 0x4a, 0x1d, 0x21, 0x42, 0x32, 0x86, 0x78, 0x20, 0xef, 0xe9, 0x7b, 0xc2, 0xd3];

/// The subscribe request byte for byte as captured (`2026-09-08-s0/*idle60.jsonl @19867.8`), spacing included.
pub const SUBSCRIBE_REQUEST: &str = r#"{"PropertySubscribeRequests":[{"ID": "0006"},{"ID": "0005"},{"ID": "000A"},{"ID": "001C"},{"ID": "0021"},{"ID": "0027"},{"ID": "0025"},{"ID": "0009"},{"ID": "0029"},{"ID": "0001"},{"ID": "0024"},{"ID": "0026"},{"ID": "000C"},{"ID": "000D"},{"ID": "000E"},{"ID": "000F"},{"ID": "0013"},{"ID": "0016" },{"ID": "0066"},{"ID": "0061"},{"ID": "005A"},{"ID": "004D"},{"ID": "001F" },{"ID": "0020" }]}"#;

pub const PROP_BATTERY: &str = "0001";
pub const PROP_CUT: &str = "0004";
pub const PROP_FEED: &str = "0007";
pub const PROP_LAST_JOB_FAILED: &str = "0009";
pub const PROP_WIDTH_MILS: &str = "000C";
pub const PROP_REMAINING_PCT: &str = "0016";
pub const PROP_READY: &str = "001F";
pub const PROP_FIRMWARE: &str = "0020";
pub const PROP_JOB_STATUS: &str = "0029";
pub const PROP_PART_NUMBER: &str = "004D";

pub fn set_request(id: &str, value: &str) -> String {
    format!(r#"{{"PropertySetRequests":[{{"ID": "{id}", "Value": "{value}"}}]}}"#)
}

/// token + u32 LE length + payload.
pub fn frame(payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(20 + payload.len());
    out.extend_from_slice(&TOKEN);
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(payload);
    out
}

/// Fragments for the request characteristic: same 3-byte headers as jobs.
pub fn request_fragments(json: &str) -> Vec<Vec<u8>> {
    crate::job::fragments(&frame(json.as_bytes()))
}

#[derive(Debug, Deserialize)]
pub struct PropertyEntry {
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "Value")]
    pub value: String,
    #[serde(rename = "Status")]
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Message {
    #[serde(rename = "PropertyGetResponses", default)]
    pub get_responses: Vec<PropertyEntry>,
}

/// Reassembles the indication byte stream into messages.
#[derive(Default)]
pub struct Reassembler {
    buf: Vec<u8>,
}

impl Reassembler {
    pub fn push(&mut self, bytes: &[u8]) -> Vec<Result<Message, String>> {
        self.buf.extend_from_slice(bytes);
        let mut out = Vec::new();
        while self.buf.len() >= 20 {
            let len = u32::from_le_bytes(self.buf[16..20].try_into().unwrap()) as usize;
            if self.buf.len() < 20 + len {
                break;
            }
            let token = self.buf[..16].to_vec();
            let payload = self.buf[20..20 + len].to_vec();
            self.buf.drain(..20 + len);
            if token != TOKEN {
                out.push(Err(format!("unexpected token {}", hex::encode(token))));
            }
            out.push(serde_json::from_slice::<Message>(&payload).map_err(|e| format!("{e}: {}", String::from_utf8_lossy(&payload))));
        }
        out
    }
}

pub type Properties = BTreeMap<String, String>;

/// What the wire tells us, in the sidecar's vocabulary. Supply name and dpi come from `crate::supply`.
#[derive(Debug, Clone, serde::Serialize, Default)]
pub struct Status {
    pub firmware_version: Option<String>,
    pub supply_part_number: Option<String>,
    pub supply_width_in: Option<f64>,
    pub supply_remaining_pct: Option<u32>,
    pub battery: Option<String>,
    pub last_job_failed: bool,
    pub last_job_status: Option<String>,
    pub ready: Option<bool>,
    pub flags: BTreeMap<String, bool>,
}

impl Status {
    pub fn from_props(p: &Properties) -> Self {
        let mut s = Status {
            firmware_version: p.get(PROP_FIRMWARE).cloned(),
            supply_part_number: p.get(PROP_PART_NUMBER).cloned(),
            supply_width_in: p.get(PROP_WIDTH_MILS).and_then(|v| v.parse::<f64>().ok()).map(|m| m / 1000.0),
            supply_remaining_pct: p.get(PROP_REMAINING_PCT).and_then(|v| v.parse().ok()),
            battery: p.get(PROP_BATTERY).cloned(),
            last_job_failed: p.get(PROP_LAST_JOB_FAILED).map(|v| v == "True").unwrap_or(false),
            last_job_status: p.get(PROP_JOB_STATUS).filter(|v| !v.is_empty()).cloned(),
            ready: p.get(PROP_READY).map(|v| v == "True"),
            flags: BTreeMap::new(),
        };
        for (k, v) in p {
            if (v == "True" || v == "False") && k != PROP_READY && k != PROP_LAST_JOB_FAILED {
                s.flags.insert(k.clone(), v == "True");
            }
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscribe_request_fragments_like_the_capture() {
        let f = request_fragments(SUBSCRIBE_REQUEST);
        assert_eq!(f.iter().map(|x| x.len()).collect::<Vec<_>>(), vec![151, 151, 121]);
        assert_eq!(&f[0][..3], &[1, 0, 0]);
        assert_eq!(&f[2][..3], &[3, 2, 0]);
        assert_eq!(&f[0][3..19], &TOKEN);
        assert_eq!(&f[0][19..23], &394u32.to_le_bytes());
    }

    #[test]
    fn set_request_matches_capture() {
        assert_eq!(set_request(PROP_FEED, "True"), r#"{"PropertySetRequests":[{"ID": "0007", "Value": "True"}]}"#);
        assert_eq!(set_request(PROP_FEED, "True").len(), 57);
    }

    #[test]
    fn reassembles_split_messages() {
        let json = br#"{"PropertyGetResponses":[{"ID":"0016","Value":"90","Status":"Successful"}]}"#;
        let framed = frame(json);
        let mut r = Reassembler::default();
        assert!(r.push(&framed[..30]).is_empty());
        let msgs = r.push(&framed[30..]);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].as_ref().unwrap().get_responses[0].value, "90");
    }
}
