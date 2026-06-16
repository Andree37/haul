use anyhow::Result;
use iroh_docs::DocTicket;
use serde::{Deserialize, Serialize};

use crate::crypto::RoomKey;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomTicket {
    pub room_key: RoomKey,
    pub room_name: String,
    pub doc_ticket: Vec<u8>, // DocTicket serialized as bytes
}

impl RoomTicket {
    pub fn new(room_key: RoomKey, room_name: String, doc_ticket: &DocTicket) -> Result<Self> {
        let doc_ticket = postcard::to_allocvec(doc_ticket)?;
        Ok(Self { room_key, room_name, doc_ticket })
    }

    pub fn doc_ticket(&self) -> Result<DocTicket> {
        Ok(postcard::from_bytes(&self.doc_ticket)?)
    }

    pub fn encode(&self) -> Result<String> {
        let bytes = postcard::to_allocvec(self)?;
        Ok(bs58::encode(bytes).into_string())
    }

    pub fn decode(s: &str) -> Result<Self> {
        let bytes = bs58::decode(s.trim()).into_vec()?;
        Ok(postcard::from_bytes(&bytes)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::generate_room_key;

    fn fake_ticket() -> RoomTicket {
        // Build a minimal RoomTicket without a real DocTicket by serializing raw bytes directly.
        // We bypass `RoomTicket::new` (which calls postcard on DocTicket) and set doc_ticket
        // to a known byte slice so the encode/decode roundtrip tests stay pure.
        RoomTicket {
            room_key: generate_room_key(),
            room_name: "test-room".to_string(),
            doc_ticket: vec![1, 2, 3, 4], // arbitrary bytes — not a real DocTicket
        }
    }

    #[test]
    fn encode_decode_roundtrip() {
        let original = fake_ticket();
        let encoded = original.encode().expect("encode succeeds");
        let decoded = RoomTicket::decode(&encoded).expect("decode succeeds");
        assert_eq!(decoded.room_key, original.room_key);
        assert_eq!(decoded.room_name, original.room_name);
        assert_eq!(decoded.doc_ticket, original.doc_ticket);
    }

    #[test]
    fn encoded_ticket_is_base58_printable() {
        let ticket = fake_ticket();
        let encoded = ticket.encode().expect("encode succeeds");
        // base58 chars are all ASCII alphanumeric, no 0/O/I/l ambiguous chars
        assert!(encoded.chars().all(|c| c.is_ascii_alphanumeric()));
        assert!(!encoded.contains('0'));
        assert!(!encoded.contains('O'));
        assert!(!encoded.contains('I'));
        assert!(!encoded.contains('l'));
    }

    #[test]
    fn decode_garbage_fails() {
        assert!(RoomTicket::decode("not-valid-base58-!!!").is_err());
    }

    #[test]
    fn decode_wrong_bytes_fails() {
        // valid base58 but not a valid postcard-encoded RoomTicket
        let junk = bs58::encode(vec![0xde, 0xad, 0xbe, 0xef]).into_string();
        assert!(RoomTicket::decode(&junk).is_err());
    }

    #[test]
    fn different_rooms_produce_different_tickets() {
        let mut t1 = fake_ticket();
        let mut t2 = fake_ticket();
        t1.room_name = "room-a".to_string();
        t2.room_name = "room-b".to_string();
        assert_ne!(t1.encode().expect("encode succeeds"), t2.encode().expect("encode succeeds"));
    }
}
