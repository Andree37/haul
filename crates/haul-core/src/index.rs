use anyhow::Result;
use futures_util::StreamExt;
use iroh::EndpointId;
use iroh_blobs::Hash;
use iroh_docs::{api::protocol::{AddrInfoOptions, ShareMode}, store::Query, NamespaceId};
use serde::{Deserialize, Serialize};

use crate::{keys, node::HaulNode};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub blob_hash: String,
    pub size: u64,
    pub modified: u64,
    pub from_node: String,
}

impl FileEntry {
    pub fn new(hash: Hash, size: u64, modified: u64, node_id: EndpointId) -> Self {
        Self {
            blob_hash: hash.to_hex().to_string(),
            size,
            modified,
            from_node: node_id.to_string(),
        }
    }

    pub fn hash(&self) -> Result<Hash> {
        self.blob_hash.parse().map_err(|e| anyhow::anyhow!("invalid blob hash: {e}"))
    }

    pub fn node_id(&self) -> Result<EndpointId> {
        self.from_node.parse().map_err(|e| anyhow::anyhow!("invalid node id: {e}"))
    }
}

pub struct RoomIndex {
    doc_id: NamespaceId,
    author: iroh_docs::AuthorId,
}

impl RoomIndex {
    pub async fn new(node: &HaulNode, doc_id: NamespaceId) -> Result<Self> {
        let author = node.docs.author_default().await?;
        Ok(Self { doc_id, author })
    }

    async fn open_doc(&self, node: &HaulNode) -> Result<iroh_docs::api::Doc> {
        node.docs
            .open(self.doc_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("room doc not open — was this room joined?"))
    }

    pub async fn put(&self, node: &HaulNode, path: &str, entry: &FileEntry) -> Result<()> {
        let doc = self.open_doc(node).await?;
        let value = serde_json::to_vec(entry)?;
        doc.set_bytes(self.author, keys::file_key(path), value).await?;
        Ok(())
    }

    pub async fn get(&self, node: &HaulNode, path: &str) -> Result<Option<FileEntry>> {
        let doc = self.open_doc(node).await?;
        let Some(entry) = doc.get_one(Query::key_exact(keys::file_key(path))).await? else {
            return Ok(None);
        };
        let content = node.blobs.blobs().get_bytes(entry.content_hash()).await?;
        Ok(Some(serde_json::from_slice(&content)?))
    }

    pub async fn list(&self, node: &HaulNode, prefix: &str) -> Result<Vec<(String, FileEntry)>> {
        let doc = self.open_doc(node).await?;
        let stream = doc.get_many(Query::key_prefix(keys::file_key(prefix))).await?;
        tokio::pin!(stream);

        let mut results = Vec::new();
        while let Some(item) = stream.next().await {
            let entry = item?;
            let path = keys::strip_file_prefix(entry.key())?.to_string();
            let content = node.blobs.blobs().get_bytes(entry.content_hash()).await?;
            let file_entry: FileEntry = serde_json::from_slice(&content)?;
            results.push((path, file_entry));
        }
        Ok(results)
    }

    pub async fn share_ticket(&self, node: &HaulNode) -> Result<iroh_docs::DocTicket> {
        let doc = self.open_doc(node).await?;
        // AddrInfoOptions::Id: ticket contains only the node ID.
        // iroh resolves the current address via DNS (node publishes to iroh.link on startup).
        // More robust than baking in relay URLs that change between process restarts.
        doc.share(ShareMode::Write, AddrInfoOptions::Id).await
    }
}
