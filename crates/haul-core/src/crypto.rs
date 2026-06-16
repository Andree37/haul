use anyhow::{bail, Result};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key, Nonce,
};
use rand::RngCore;

pub const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;

pub type RoomKey = [u8; KEY_LEN];

pub fn generate_room_key() -> RoomKey {
    let mut key = [0u8; KEY_LEN];
    rand::thread_rng().fill_bytes(&mut key);
    key
}

pub fn encrypt(key: &RoomKey, plaintext: &[u8]) -> Result<Vec<u8>> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| anyhow::anyhow!("encrypt failed: {e}"))?;

    let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

pub fn decrypt(key: &RoomKey, data: &[u8]) -> Result<Vec<u8>> {
    if data.len() < NONCE_LEN {
        bail!("data too short to contain nonce");
    }
    let (nonce_bytes, ciphertext) = data.split_at(NONCE_LEN);
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow::anyhow!("decrypt failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let key = generate_room_key();
        let plaintext = b"hello haul";
        let ciphertext = encrypt(&key, plaintext).expect("encrypt succeeds");
        let recovered = decrypt(&key, &ciphertext).expect("decrypt succeeds");
        assert_eq!(recovered, plaintext);
    }

    #[test]
    fn encrypt_produces_different_ciphertext_each_call() {
        let key = generate_room_key();
        let plaintext = b"same plaintext";
        let c1 = encrypt(&key, plaintext).expect("encrypt succeeds");
        let c2 = encrypt(&key, plaintext).expect("encrypt succeeds");
        // nonces differ → ciphertexts differ
        assert_ne!(c1, c2);
    }

    #[test]
    fn decrypt_wrong_key_fails() {
        let key = generate_room_key();
        let other_key = generate_room_key();
        let ciphertext = encrypt(&key, b"secret").expect("encrypt succeeds");
        assert!(decrypt(&other_key, &ciphertext).is_err());
    }

    #[test]
    fn decrypt_truncated_data_fails() {
        let key = generate_room_key();
        // fewer bytes than NONCE_LEN
        assert!(decrypt(&key, &[0u8; 4]).is_err());
    }

    #[test]
    fn decrypt_tampered_ciphertext_fails() {
        let key = generate_room_key();
        let mut ciphertext = encrypt(&key, b"secret data").expect("encrypt succeeds");
        // flip a byte in the ciphertext portion (after nonce)
        let last = ciphertext.len() - 1;
        ciphertext[last] ^= 0xff;
        assert!(decrypt(&key, &ciphertext).is_err());
    }

    #[test]
    fn encrypt_empty_plaintext() {
        let key = generate_room_key();
        let ciphertext = encrypt(&key, b"").expect("encrypt succeeds");
        let recovered = decrypt(&key, &ciphertext).expect("decrypt succeeds");
        assert_eq!(recovered, b"");
    }

    #[test]
    fn generated_keys_are_unique() {
        let k1 = generate_room_key();
        let k2 = generate_room_key();
        assert_ne!(k1, k2);
    }
}
