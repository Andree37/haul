use anyhow::Result;
use directories::ProjectDirs;
use iroh::SecretKey;
use std::path::{Path, PathBuf};
use tokio::fs;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use crate::constants::{APP_NAME, APP_ORG, APP_QUALIFIER};

pub fn data_dir() -> Result<PathBuf> {
    ProjectDirs::from(APP_ORG, APP_QUALIFIER, APP_NAME)
        .map(|dirs| dirs.data_dir().to_path_buf())
        .ok_or_else(|| anyhow::anyhow!("cannot determine platform data directory"))
}

pub async fn load_or_create_secret_key(path: &Path) -> Result<SecretKey> {
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
