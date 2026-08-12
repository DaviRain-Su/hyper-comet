//! Local Launch Studio persistence.
//!
//! The dapp used `localStorage` under `proofship.launches.v1`; the native
//! engine writes the same launch/chat message shapes to
//! `{data_dir}/studio/launches.json`. Writes are capped and atomic so the UI can
//! treat this as durable local state without involving the synced workspace doc.

use std::path::{Path, PathBuf};

use comet_proto::StudioLaunch;

const MAX_LAUNCHES: usize = 20;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct StudioStore {
    file: PathBuf,
}

impl StudioStore {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            file: data_dir.join("studio").join("launches.json"),
        }
    }

    pub fn path(&self) -> &Path {
        &self.file
    }

    pub fn load(&self) -> Result<Vec<StudioLaunch>, StoreError> {
        match std::fs::read_to_string(&self.file) {
            Ok(raw) => {
                let mut launches: Vec<StudioLaunch> = serde_json::from_str(&raw)?;
                launches.truncate(MAX_LAUNCHES);
                Ok(launches)
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(err) => Err(err.into()),
        }
    }

    pub fn save(&self, launches: &[StudioLaunch]) -> Result<Vec<StudioLaunch>, StoreError> {
        let capped: Vec<_> = launches.iter().take(MAX_LAUNCHES).cloned().collect();
        if let Some(parent) = self.file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = self.file.with_extension("json.tmp");
        let json = serde_json::to_vec(&capped)?;
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, &self.file)?;
        Ok(capped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_roundtrip_caps_twenty() {
        let dir = tempfile::tempdir().unwrap();
        let store = StudioStore::new(dir.path());
        let launches: Vec<_> = (0..25)
            .map(|i| StudioLaunch {
                id: format!("id-{i}"),
                title: format!("Launch {i}"),
                created_at: "2026-08-12T00:00:00Z".into(),
                msgs: Vec::new(),
                fields: None,
                program: None,
                source: None,
            })
            .collect();
        let saved = store.save(&launches).unwrap();
        assert_eq!(saved.len(), 20);
        let loaded = store.load().unwrap();
        assert_eq!(loaded.len(), 20);
        assert_eq!(loaded[0].id, "id-0");
        assert_eq!(loaded[19].id, "id-19");
        assert!(store.path().exists());
    }
}
