//! Local EVM network presets persisted under `{data_dir}/studio/networks.json`.

use std::path::{Path, PathBuf};

use comet_proto::{EvmNetwork, builtin_networks};

#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("cannot remove built-in network {0}")]
    Builtin(String),
}

#[derive(Debug, Clone)]
pub struct NetworkStore {
    file: PathBuf,
}

impl NetworkStore {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            file: data_dir.join("studio").join("networks.json"),
        }
    }

    pub fn path(&self) -> &Path {
        &self.file
    }

    pub fn load(&self) -> Result<Vec<EvmNetwork>, NetworkError> {
        match std::fs::read_to_string(&self.file) {
            Ok(raw) => {
                let networks: Vec<EvmNetwork> = serde_json::from_str(&raw)?;
                Ok(merge_builtins(networks))
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(builtin_networks()),
            Err(err) => Err(err.into()),
        }
    }

    pub fn save(&self, networks: &[EvmNetwork]) -> Result<Vec<EvmNetwork>, NetworkError> {
        let merged = ensure_builtins(networks.to_vec());
        if let Some(parent) = self.file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = self.file.with_extension("json.tmp");
        let json = serde_json::to_vec(&merged)?;
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, &self.file)?;
        Ok(merged)
    }

    pub fn upsert(&self, network: EvmNetwork) -> Result<Vec<EvmNetwork>, NetworkError> {
        validate_network(&network)?;
        let mut network = network;
        if is_builtin_id(&network.id) {
            network.builtin = true;
            if let Some(preset) = builtin_networks().into_iter().find(|n| n.id == network.id) {
                network.chain_id = preset.chain_id;
            }
        }
        let mut networks = self.load()?;
        if let Some(ix) = networks.iter().position(|n| n.id == network.id) {
            networks[ix] = network;
        } else {
            networks.push(network);
        }
        self.save(&networks).map_err(NetworkError::from)
    }

    pub fn remove(&self, id: &str) -> Result<Vec<EvmNetwork>, NetworkError> {
        if is_builtin_id(id) {
            return Err(NetworkError::Builtin(id.to_string()));
        }
        let mut networks = self.load()?;
        networks.retain(|n| n.id != id);
        self.save(&networks).map_err(NetworkError::from)
    }
}

fn is_builtin_id(id: &str) -> bool {
    builtin_networks().iter().any(|n| n.id == id)
}

/// After load: force `builtin` on preset rows; prepend any missing built-ins.
fn merge_builtins(networks: Vec<EvmNetwork>) -> Vec<EvmNetwork> {
    let mut networks = networks;
    for network in &mut networks {
        if is_builtin_id(&network.id) {
            network.builtin = true;
        }
    }
    for builtin in builtin_networks().iter().rev() {
        if !networks.iter().any(|n| n.id == builtin.id) {
            networks.insert(0, builtin.clone());
        }
    }
    networks
}

/// Before save: never drop built-in ids — re-insert defaults when absent.
fn ensure_builtins(networks: Vec<EvmNetwork>) -> Vec<EvmNetwork> {
    merge_builtins(networks)
}

fn validate_network(network: &EvmNetwork) -> Result<(), NetworkError> {
    if network.id.is_empty()
        || !network
            .id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(invalid_input("id must be non-empty and match [a-z0-9-]+"));
    }
    if network.name.trim().is_empty() {
        return Err(invalid_input("name must be non-empty"));
    }
    if network.chain_id == 0 {
        return Err(invalid_input("chainId must be greater than 0"));
    }
    if !network.rpc_url.starts_with("http://") && !network.rpc_url.starts_with("https://") {
        return Err(invalid_input("rpcUrl must start with http:// or https://"));
    }
    if network.currency_symbol.trim().is_empty() {
        return Err(invalid_input("currencySymbol must be non-empty"));
    }
    Ok(())
}

fn invalid_input(message: impl Into<String>) -> NetworkError {
    NetworkError::Io(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        message.into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn custom() -> EvmNetwork {
        EvmNetwork {
            id: "my-net".into(),
            name: "My Net".into(),
            chain_id: 42,
            rpc_url: "https://rpc.example.com".into(),
            explorer_url: Some("https://explorer.example.com".into()),
            currency_symbol: "ETH".into(),
            builtin: false,
        }
    }

    #[test]
    fn missing_file_yields_xlayer_first_builtins() {
        let dir = tempfile::tempdir().unwrap();
        let store = NetworkStore::new(dir.path());
        let loaded = store.load().unwrap();
        assert!(loaded.len() >= 2);
        assert!(loaded.iter().all(|n| n.builtin));
        assert_eq!(loaded[0].id, "xlayer-testnet");
        assert_eq!(loaded[1].id, "xlayer-mainnet");
        assert!(loaded.iter().any(|n| n.id == "ethereum-sepolia"));
        assert!(loaded.iter().any(|n| n.id == "base-sepolia"));
    }

    #[test]
    fn custom_network_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let store = NetworkStore::new(dir.path());
        let custom = custom();
        let saved = store.save(&[custom.clone()]).unwrap();
        assert!(saved.iter().any(|n| n.id == "my-net"));
        let loaded = store.load().unwrap();
        let got = loaded.iter().find(|n| n.id == "my-net").unwrap();
        assert_eq!(got, &custom);
        assert!(store.path().exists());
    }

    #[test]
    fn cannot_remove_builtin() {
        let dir = tempfile::tempdir().unwrap();
        let store = NetworkStore::new(dir.path());
        store.load().unwrap();
        let err = store.remove("xlayer-testnet").unwrap_err();
        assert!(matches!(err, NetworkError::Builtin(id) if id == "xlayer-testnet"));
    }

    #[test]
    fn upsert_custom_then_load() {
        let dir = tempfile::tempdir().unwrap();
        let store = NetworkStore::new(dir.path());
        let custom = custom();
        store.upsert(custom.clone()).unwrap();
        let loaded = store.load().unwrap();
        assert!(loaded.iter().any(|n| n.id == "my-net"));
        assert_eq!(loaded.iter().find(|n| n.id == "my-net").unwrap(), &custom);
    }

    #[test]
    fn builtin_upsert_keeps_preset_chain_id() {
        let dir = tempfile::tempdir().unwrap();
        let store = NetworkStore::new(dir.path());
        let mut mainnet = builtin_networks()
            .into_iter()
            .find(|n| n.id == "xlayer-mainnet")
            .unwrap();
        mainnet.chain_id = 1;
        mainnet.rpc_url = "https://example.invalid".into();
        store.upsert(mainnet).unwrap();
        let loaded = store.load().unwrap();
        let row = loaded.iter().find(|n| n.id == "xlayer-mainnet").unwrap();
        assert_eq!(row.chain_id, 196);
        assert_eq!(row.rpc_url, "https://example.invalid");
    }

    #[test]
    fn old_file_with_only_custom_still_gets_builtins_merged_in() {
        let dir = tempfile::tempdir().unwrap();
        let store = NetworkStore::new(dir.path());
        std::fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        std::fs::write(
            store.path(),
            r#"[{"id":"my-net","name":"My Net","chainId":42,"rpcUrl":"https://rpc.example.com","currencySymbol":"ETH"}]"#,
        )
        .unwrap();
        let loaded = store.load().unwrap();
        // 4 builtins (X Layer test/main + Sepolia + Base Sepolia) + 1 custom
        assert_eq!(loaded.len(), 5);
        assert_eq!(loaded[0].id, "xlayer-testnet");
        assert_eq!(loaded[1].id, "xlayer-mainnet");
        assert!(loaded.iter().any(|n| n.id == "my-net"));
    }
}
