use anyhow::Result;
use iroh::{endpoint::presets, Endpoint, EndpointId};
use iroh_blobs::{store::fs::FsStore, BlobsProtocol};
use iroh_docs::protocol::Docs;
use iroh_gossip::net::Gossip;
use iroh::protocol::Router;
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;

use crate::constants::{BLOBS_DIR, DOCS_DIR, NODE_KEY_FILE};
use crate::identity::load_or_create_secret_key;

pub struct HaulNode {
    pub endpoint: Endpoint,
    pub blobs: FsStore,
    pub docs: Docs,
    router: Router,
}

impl HaulNode {
    pub async fn spawn(data_dir: &PathBuf) -> Result<Self> {
        fs::create_dir_all(data_dir).await?;

        let secret_key = load_or_create_secret_key(&data_dir.join(NODE_KEY_FILE)).await?;
        tracing::info!("node id: {}", secret_key.public());

        let blobs_dir = data_dir.join(BLOBS_DIR);
        let docs_dir = data_dir.join(DOCS_DIR);
        fs::create_dir_all(&blobs_dir).await?;
        fs::create_dir_all(&docs_dir).await?;

        let endpoint = Endpoint::builder(presets::N0)
            .secret_key(secret_key)
            .bind()
            .await?;

        // TODO(daemon): poll until relay is ready so tickets include a relay URL.
        // Needed because each CLI invocation starts a cold node. A persistent daemon
        // keeps the relay connection open and this loop disappears.
        let relay_deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        loop {
            if endpoint.addr().relay_urls().next().is_some() {
                break;
            }
            if tokio::time::Instant::now() >= relay_deadline {
                tracing::warn!("no relay connection after 10s — ticket addresses may be incomplete");
                break;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        let blobs = FsStore::load(&blobs_dir).await?;
        let gossip = Gossip::builder().spawn(endpoint.clone());
        let docs = Docs::persistent(docs_dir)
            .spawn(endpoint.clone(), blobs.clone().into(), gossip.clone())
            .await?;

        let router = Router::builder(endpoint.clone())
            .accept(iroh_blobs::ALPN, BlobsProtocol::new(&blobs, None))
            .accept(iroh_gossip::ALPN, gossip)
            .accept(iroh_docs::ALPN, docs.clone())
            .spawn();

        Ok(Self { endpoint, blobs, docs, router })
    }

    pub fn node_id(&self) -> EndpointId {
        self.endpoint.id()
    }

    pub async fn shutdown(self) -> Result<()> {
        self.router.shutdown().await?;
        Ok(())
    }
}
