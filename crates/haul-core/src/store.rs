use anyhow::Result;
use iroh::EndpointId;
use iroh_blobs::Hash;
use std::path::Path;

use crate::{
    crypto::{decrypt, encrypt, RoomKey},
    node::HaulNode,
};

pub struct BlobStore<'a> {
    node: &'a HaulNode,
}

impl<'a> BlobStore<'a> {
    pub fn new(node: &'a HaulNode) -> Self {
        Self { node }
    }

    pub async fn add_file_encrypted(&self, key: &RoomKey, path: &Path) -> Result<(Hash, u64, u64)> {
        let plaintext = tokio::fs::read(path).await?;
        let modified = tokio::fs::metadata(path)
            .await?
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        let ciphertext = encrypt(key, &plaintext)?;
        let size = ciphertext.len() as u64;
        let tag = self.node.blobs.blobs().add_bytes(ciphertext).await?;
        Ok((tag.hash, size, modified))
    }

    pub async fn fetch_from_peer(&self, hash: Hash, peer: EndpointId) -> Result<()> {
        if self.node.blobs.blobs().has(hash).await? {
            return Ok(());
        }
        self.node
            .blobs
            .downloader(&self.node.endpoint)
            .download(hash, vec![peer])
            .await?;
        Ok(())
    }

    pub async fn export_decrypted(&self, key: &RoomKey, hash: Hash, dest: &Path) -> Result<()> {
        let ciphertext = self.node.blobs.blobs().get_bytes(hash).await?;
        let plaintext = decrypt(key, &ciphertext)?;
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(dest, plaintext).await?;
        Ok(())
    }
}
