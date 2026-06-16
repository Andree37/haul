pub mod constants;
pub mod crypto;
pub mod index;
pub mod keys;
pub mod node;
pub mod room;
pub mod store;
pub mod ticket;

pub use node::{data_dir, HaulNode};
pub use room::{Room, RoomRegistry};
pub use ticket::RoomTicket;

use anyhow::Result;
use futures_util::StreamExt;
use index::{FileEntry, RoomIndex};
use iroh_docs::engine::LiveEvent;
use std::path::{Path, PathBuf};
use std::time::Duration;
use store::BlobStore;
use walkdir::WalkDir;

pub struct Haul {
    pub node: HaulNode,
    pub rooms: RoomRegistry,
    data_dir: PathBuf,
}

impl Haul {
    pub async fn open(data_dir: PathBuf) -> Result<Self> {
        let node = HaulNode::spawn(&data_dir).await?;
        let rooms = RoomRegistry::load(&data_dir).await?;
        Ok(Self { node, rooms, data_dir })
    }

    async fn save_rooms(&self) -> Result<()> {
        self.rooms.save(&self.data_dir).await
    }

    pub async fn room_create(&mut self, name: &str) -> Result<RoomTicket> {
        let key = crypto::generate_room_key();
        let doc = self.node.docs.create().await?;
        let doc_id = doc.id();

        let index = RoomIndex::new(&self.node, doc_id).await?;
        let doc_ticket = index.share_ticket(&self.node).await?;

        self.rooms.insert(Room::new(name.to_string(), key, doc_id));
        self.save_rooms().await?;

        RoomTicket::new(key, name.to_string(), &doc_ticket)
    }

    pub async fn room_join(&mut self, ticket_str: &str) -> Result<(String, usize)> {
        let ticket = RoomTicket::decode(ticket_str)?;
        let doc_ticket = ticket.doc_ticket()?;
        let (doc, mut events) = self.node.docs.import_and_subscribe(doc_ticket).await?;

        // TODO(daemon): blocking sync wait. With a daemon the node stays alive and
        // sync happens in the background — callers just query local state. Remove this
        // timeout when join becomes a fire-and-forget RPC to a running daemon.
        let mut synced = 0usize;
        match tokio::time::timeout(Duration::from_secs(30), async {
            while let Some(ev) = events.next().await {
                match ev? {
                    LiveEvent::NeighborUp(peer) => tracing::info!("peer connected: {peer}"),
                    LiveEvent::InsertRemote { .. } => synced += 1,
                    // SyncFinished = CRDT entries replicated, but value blobs not yet downloaded.
                    // PendingContentReady = all value blobs available locally — safe to query.
                    LiveEvent::PendingContentReady => return Ok::<_, anyhow::Error>(()),
                    _ => {}
                }
            }
            Ok(())
        })
        .await
        {
            Ok(Err(e)) => tracing::warn!("sync error during join: {e:#}"),
            Err(_) => tracing::debug!("sync timed out — no peer online"),
            Ok(Ok(())) => {}
        }

        self.rooms.insert(Room::new(ticket.room_name.clone(), ticket.room_key, doc.id()));
        self.save_rooms().await?;
        Ok((ticket.room_name, synced))
    }

    pub async fn room_invite(&self, room_name: &str) -> Result<RoomTicket> {
        let room = self.get_room(room_name)?;
        let index = RoomIndex::new(&self.node, room.namespace_id()).await?;
        let doc_ticket = index.share_ticket(&self.node).await?;
        RoomTicket::new(room.key, room.name.clone(), &doc_ticket)
    }

    pub async fn add(&self, path: &Path, room_name: &str) -> Result<u64> {
        let room = self.get_room(room_name)?;
        let index = RoomIndex::new(&self.node, room.namespace_id()).await?;
        let bs = BlobStore::new(&self.node);
        let node_id = self.node.node_id();
        let mut count = 0u64;

        if path.is_file() {
            let base = add_base(path);
            self.add_one_file(&bs, &index, path, base, &room.key, node_id).await?;
            count += 1;
        } else {
            let base = add_base(path);
            for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    self.add_one_file(&bs, &index, entry.path(), base, &room.key, node_id).await?;
                    count += 1;
                }
            }
        }
        Ok(count)
    }

    async fn add_one_file(
        &self,
        bs: &BlobStore<'_>,
        index: &RoomIndex,
        file_path: &Path,
        base: &Path,
        key: &crypto::RoomKey,
        node_id: iroh::EndpointId,
    ) -> Result<()> {
        let (hash, size, modified) = bs.add_file_encrypted(key, file_path).await?;
        let rel = relative_path(file_path, base)?.to_string_lossy().to_string();
        index.put(&self.node, &rel, &FileEntry::new(hash, size, modified, node_id)).await?;
        tracing::info!("added {rel} ({size} encrypted bytes)");
        Ok(())
    }

    pub async fn ls(&self, room_name: &str, prefix: &str) -> Result<Vec<(String, FileEntry)>> {
        let room = self.get_room(room_name)?;
        let index = RoomIndex::new(&self.node, room.namespace_id()).await?;
        index.list(&self.node, prefix).await
    }

    pub async fn get(&self, room_name: &str, file_path: &str, out_dir: &Path) -> Result<()> {
        let room = self.get_room(room_name)?;
        let index = RoomIndex::new(&self.node, room.namespace_id()).await?;
        let bs = BlobStore::new(&self.node);

        let entry = index
            .get(&self.node, file_path)
            .await?
            .ok_or_else(|| anyhow::anyhow!("'{file_path}' not found in room '{room_name}'"))?;

        let hash = entry.hash()?;
        let peer = entry.node_id()?;
        bs.fetch_from_peer(hash, peer).await?;
        bs.export_decrypted(&room.key, hash, &out_dir.join(file_path)).await?;
        Ok(())
    }

    /// Put all known room docs into active sync mode so incoming peer requests are accepted.
    /// TODO(daemon): fold into Haul::open via SyncMode flag — daemon always Active, one-shot commands Passive.
    pub async fn start_syncing_rooms(&self) -> Result<()> {
        for room in self.rooms.list() {
            if let Some(doc) = self.node.docs.open(room.namespace_id()).await? {
                doc.start_sync(vec![]).await?;
            }
        }
        Ok(())
    }

    pub async fn shutdown(self) -> Result<()> {
        self.node.shutdown().await
    }

    fn get_room(&self, name: &str) -> Result<&Room> {
        self.rooms
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("room '{name}' not found — run `haul rooms` to list"))
    }
}

// base dir for strip_prefix when adding a path.
// file:  /tmp/report.pdf   → base = /tmp       → stored as "report.pdf"
// dir:   /tmp/testdir      → base = /tmp       → stored as "testdir/a.txt"
fn add_base(path: &Path) -> &Path {
    match path.parent() {
        Some(p) => p,
        None => path, // root path has no parent; use itself
    }
}

// path of file relative to base; errors if file_path is not under base.
fn relative_path<'a>(file_path: &'a Path, base: &Path) -> Result<&'a Path> {
    file_path.strip_prefix(base).map_err(|_| {
        anyhow::anyhow!("'{}' is not under base '{}'", file_path.display(), base.display())
    })
}

#[cfg(test)]
mod path_tests {
    use super::*;
    use std::path::Path;

    // --- add_base ---

    #[test]
    fn add_base_single_file_gives_parent_dir() {
        let p = Path::new("/tmp/report.pdf");
        assert_eq!(add_base(p), Path::new("/tmp"));
    }

    #[test]
    fn add_base_directory_gives_parent_dir() {
        // This is the critical invariant: folders use parent, NOT the folder itself.
        // Using the folder as base caused files to lose their dir component (the bug).
        let p = Path::new("/tmp/testdir");
        assert_eq!(add_base(p), Path::new("/tmp"));
    }

    #[test]
    fn add_base_nested_dir_gives_immediate_parent() {
        let p = Path::new("/home/user/docs/work");
        assert_eq!(add_base(p), Path::new("/home/user/docs"));
    }

    // --- relative_path ---

    #[test]
    fn relative_path_single_file() {
        let file = Path::new("/tmp/report.pdf");
        let base = add_base(file); // /tmp
        assert_eq!(relative_path(file, base).expect("file is under base"), Path::new("report.pdf"));
    }

    #[test]
    fn relative_path_file_in_folder_preserves_dir_component() {
        // The bug: base was /tmp/testdir, so result was "a.txt" not "testdir/a.txt".
        // Fix: base = /tmp (parent of /tmp/testdir), so result = "testdir/a.txt".
        let dir = Path::new("/tmp/testdir");
        let base = add_base(dir); // /tmp
        let file = dir.join("a.txt"); // /tmp/testdir/a.txt
        assert_eq!(relative_path(&file, base).expect("file is under base"), Path::new("testdir/a.txt"));
    }

    #[test]
    fn relative_path_nested_file_in_folder() {
        let dir = Path::new("/tmp/testdir");
        let base = add_base(dir); // /tmp
        let file = dir.join("sub/deep/b.txt"); // /tmp/testdir/sub/deep/b.txt
        assert_eq!(relative_path(&file, base).expect("file is under base"), Path::new("testdir/sub/deep/b.txt"));
    }

    #[test]
    fn relative_path_buggy_base_loses_dir_component() {
        // Documents the OLD (wrong) behaviour so regressions are obvious.
        let dir = Path::new("/tmp/testdir");
        let file = dir.join("a.txt");
        // wrong base = the dir itself (the bug); strip still succeeds → "a.txt"
        let wrong = relative_path(&file, dir).expect("file is under dir");
        assert_eq!(wrong, Path::new("a.txt")); // loses "testdir/" — wrong!
        // correct base = parent of the dir
        let correct = relative_path(&file, add_base(dir)).expect("file is under parent");
        assert_eq!(correct, Path::new("testdir/a.txt")); // preserves dir — right!
    }

    #[test]
    fn relative_path_errors_when_strip_fails() {
        // file_path not under base → error, not silent wrong path
        let file = Path::new("/other/path/file.txt");
        let base = Path::new("/tmp");
        assert!(relative_path(file, base).is_err());
    }
}
