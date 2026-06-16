use anyhow::Result;
use directories::ProjectDirs;
use iroh::{endpoint::presets, Endpoint, EndpointId, SecretKey};
use iroh_blobs::{store::fs::FsStore, BlobsProtocol};
use iroh_docs::protocol::Docs;
use iroh_gossip::net::Gossip;
use iroh::protocol::Router;
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use crate::constants::{APP_NAME, APP_ORG, APP_QUALIFIER, BLOBS_DIR, DOCS_DIR, NODE_KEY_FILE};

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

async fn load_or_create_secret_key(path: &PathBuf) -> Result<SecretKey> {
    match fs::read(path).await {
        Ok(bytes) => {
            let arr: [u8; 32] = bytes
                .try_into()
                .map_err(|_| anyhow::anyhow!("node key file is corrupt — delete it and re-run `haul status`"))?;
            Ok(SecretKey::from_bytes(&arr))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let key = SecretKey::generate();
            fs::write(path, key.to_bytes()).await?;
            #[cfg(unix)]
            fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).await?;
            Ok(key)
        }
        Err(e) => Err(e.into()),
    }
}

pub fn data_dir() -> Result<PathBuf> {
    ProjectDirs::from(APP_ORG, APP_QUALIFIER, APP_NAME)
        .map(|dirs| dirs.data_dir().to_path_buf())
        .ok_or_else(|| anyhow::anyhow!("cannot determine platform data directory"))
}
