use anyhow::Result;

use crate::constants::{DOC_PREFIX_FILE, DOC_PREFIX_MEMBER};

pub fn file_key(path: &str) -> Vec<u8> {
    format!("{DOC_PREFIX_FILE}{path}").into_bytes()
}

pub fn member_key(node_id: &str) -> Vec<u8> {
    format!("{DOC_PREFIX_MEMBER}{node_id}").into_bytes()
}

pub fn strip_file_prefix(key: &[u8]) -> Result<&str> {
    let s = std::str::from_utf8(key)
        .map_err(|e| anyhow::anyhow!("doc key is not valid UTF-8: {e}"))?;
    Ok(match s.strip_prefix(DOC_PREFIX_FILE) {
        Some(stripped) => stripped,
        None => s,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{DOC_PREFIX_FILE, DOC_PREFIX_MEMBER};

    #[test]
    fn file_key_has_correct_prefix() {
        let key = file_key("photos/cat.jpg");
        assert!(key.starts_with(DOC_PREFIX_FILE.as_bytes()));
        assert!(key.ends_with(b"photos/cat.jpg"));
    }

    #[test]
    fn member_key_has_correct_prefix() {
        let key = member_key("abc123");
        assert!(key.starts_with(DOC_PREFIX_MEMBER.as_bytes()));
        assert!(key.ends_with(b"abc123"));
    }

    #[test]
    fn strip_file_prefix_roundtrip() {
        let path = "docs/readme.txt";
        let key = file_key(path);
        assert_eq!(strip_file_prefix(&key).expect("valid UTF-8 key"), path);
    }

    #[test]
    fn strip_file_prefix_no_prefix_returns_full() {
        // key without the file prefix — returns whole string unchanged
        let raw = b"nomatch/something";
        assert_eq!(strip_file_prefix(raw).expect("valid UTF-8 key"), "nomatch/something");
    }

    #[test]
    fn strip_file_prefix_invalid_utf8_errors() {
        let bad = b"\xff\xfe invalid utf8";
        assert!(strip_file_prefix(bad).is_err());
    }

    #[test]
    fn file_key_empty_path() {
        let key = file_key("");
        assert_eq!(key, DOC_PREFIX_FILE.as_bytes());
        assert_eq!(strip_file_prefix(&key).expect("valid UTF-8 key"), "");
    }

    #[test]
    fn file_and_member_keys_dont_collide() {
        // same suffix must not collide between namespaces
        let fk = file_key("node123");
        let mk = member_key("node123");
        assert_ne!(fk, mk);
    }
}
