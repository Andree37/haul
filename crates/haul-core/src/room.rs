use anyhow::Result;
use iroh_docs::NamespaceId;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};
use tokio::fs;

use crate::{constants::ROOMS_FILE, crypto::RoomKey};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub name: String,
    pub key: RoomKey,
    pub doc_id: [u8; 32],
}

impl Room {
    pub fn new(name: String, key: RoomKey, doc_id: NamespaceId) -> Self {
        Self {
            name,
            key,
            doc_id: *doc_id.as_bytes(),
        }
    }

    pub fn namespace_id(&self) -> NamespaceId {
        NamespaceId::from(self.doc_id)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RoomRegistry {
    rooms: HashMap<String, Room>,
}

impl RoomRegistry {
    pub async fn load(data_dir: &Path) -> Result<Self> {
        let path = data_dir.join(ROOMS_FILE);
        if path.exists() {
            let data = fs::read(&path).await?;
            Ok(serde_json::from_slice(&data)?)
        } else {
            Ok(Self::default())
        }
    }

    pub async fn save(&self, data_dir: &Path) -> Result<()> {
        let path = data_dir.join(ROOMS_FILE);
        let data = serde_json::to_vec_pretty(self)?;
        fs::write(path, data).await?;
        Ok(())
    }

    pub fn insert(&mut self, room: Room) {
        self.rooms.insert(room.name.clone(), room);
    }

    pub fn get(&self, name: &str) -> Option<&Room> {
        self.rooms.get(name)
    }

    pub fn list(&self) -> impl Iterator<Item = &Room> {
        self.rooms.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::generate_room_key;

    fn make_namespace() -> NamespaceId {
        NamespaceId::from([42u8; 32])
    }

    #[test]
    fn room_namespace_id_roundtrip() {
        let ns = make_namespace();
        let room = Room::new("myroom".to_string(), generate_room_key(), ns);
        assert_eq!(room.namespace_id(), ns);
    }

    #[test]
    fn registry_insert_and_get() {
        let mut reg = RoomRegistry::default();
        let room = Room::new("alpha".to_string(), generate_room_key(), make_namespace());
        reg.insert(room.clone());
        let found = reg.get("alpha").expect("alpha was just inserted");
        assert_eq!(found.name, "alpha");
        assert_eq!(found.doc_id, room.doc_id);
    }

    #[test]
    fn registry_get_missing_returns_none() {
        let reg = RoomRegistry::default();
        assert!(reg.get("no-such-room").is_none());
    }

    #[test]
    fn registry_insert_overwrites() {
        let mut reg = RoomRegistry::default();
        let key1 = generate_room_key();
        let key2 = generate_room_key();
        reg.insert(Room::new("r".to_string(), key1, make_namespace()));
        reg.insert(Room::new("r".to_string(), key2, make_namespace()));
        assert_eq!(reg.get("r").expect("r was inserted twice").key, key2);
    }

    #[test]
    fn registry_list_all() {
        let mut reg = RoomRegistry::default();
        reg.insert(Room::new("a".to_string(), generate_room_key(), make_namespace()));
        reg.insert(Room::new("b".to_string(), generate_room_key(), make_namespace()));
        let names: Vec<_> = reg.list().map(|r| r.name.as_str()).collect();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
    }
}
