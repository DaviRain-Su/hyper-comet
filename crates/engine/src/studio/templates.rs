//! Bundled + on-disk Studio templates (`proofship/templates/`).

use std::path::{Path, PathBuf};

use comet_proto::StudioTemplate;
use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unknown template {0}")]
    Unknown(String),
}

#[derive(Debug, Clone)]
pub struct TemplateStore {
    roots: Vec<PathBuf>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestFile {
    id: String,
    name: String,
    description: String,
    module: String,
    preferred_network_id: String,
    nl_seed: String,
    #[serde(default)]
    ctor_sig: Option<String>,
    #[serde(default)]
    ctor_hints: Vec<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    design: Option<String>,
    #[serde(default)]
    source_file: Option<String>,
}

impl TemplateStore {
    pub fn new(extra_roots: impl IntoIterator<Item = PathBuf>) -> Self {
        let mut roots = Vec::new();
        if let Ok(env) = std::env::var("PROOFSHIP_TEMPLATES") {
            let p = PathBuf::from(env.trim());
            if !p.as_os_str().is_empty() {
                roots.push(p);
            }
        }
        roots.extend(extra_roots);
        // Repo checkout layout when running from source.
        if let Ok(cwd) = std::env::current_dir() {
            let candidate = cwd.join("proofship").join("templates");
            if candidate.is_dir() {
                roots.push(candidate);
            }
            let candidate = cwd.join("templates");
            if candidate.is_dir() {
                roots.push(candidate);
            }
        }
        Self { roots }
    }

    pub fn list(&self) -> Result<Vec<StudioTemplate>, TemplateError> {
        let mut out = bundled_templates();
        let mut seen: std::collections::HashSet<String> =
            out.iter().map(|t| t.id.clone()).collect();
        for root in &self.roots {
            for tmpl in load_dir(root)? {
                if seen.insert(tmpl.id.clone()) {
                    out.push(tmpl);
                }
            }
        }
        // X Layer–oriented templates first.
        out.sort_by(|a, b| {
            let a_x = a.preferred_network_id.starts_with("xlayer") as i32;
            let b_x = b.preferred_network_id.starts_with("xlayer") as i32;
            b_x.cmp(&a_x).then_with(|| a.name.cmp(&b.name))
        });
        Ok(out)
    }

    pub fn get(&self, id: &str) -> Result<StudioTemplate, TemplateError> {
        self.list()?
            .into_iter()
            .find(|t| t.id == id)
            .ok_or_else(|| TemplateError::Unknown(id.to_string()))
    }
}

fn load_dir(root: &Path) -> Result<Vec<StudioTemplate>, TemplateError> {
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('_') {
            continue;
        }
        let manifest_path = entry.path().join("template.json");
        if !manifest_path.is_file() {
            continue;
        }
        let raw = std::fs::read_to_string(&manifest_path)?;
        let manifest: ManifestFile = serde_json::from_str(&raw)?;
        let source_name = manifest
            .source_file
            .unwrap_or_else(|| "program.lean".into());
        let source_path = entry.path().join(&source_name);
        let source = if source_path.is_file() {
            Some(std::fs::read_to_string(source_path)?)
        } else {
            None
        };
        let abi_path = entry.path().join("abi.json");
        let abi_json = if abi_path.is_file() {
            Some(std::fs::read_to_string(abi_path)?)
        } else {
            None
        };
        out.push(StudioTemplate {
            id: manifest.id,
            name: manifest.name,
            description: manifest.description,
            module: manifest.module,
            preferred_network_id: manifest.preferred_network_id,
            nl_seed: manifest.nl_seed,
            ctor_sig: manifest.ctor_sig,
            ctor_hints: manifest.ctor_hints,
            tags: manifest.tags,
            design: manifest.design,
            source,
            abi_json,
        });
    }
    Ok(out)
}

/// Always-available golden template (works even if cwd has no `proofship/templates`).
fn bundled_templates() -> Vec<StudioTemplate> {
    vec![StudioTemplate {
        id: "rwa-share-registry".into(),
        name: "RWA Share Registry".into(),
        description: "Onchain share registry with allowlist, per-tx cap, and rolling window cap. X Layer first."
            .into(),
        module: "RwaShareRegistry".into(),
        preferred_network_id: "xlayer-testnet".into(),
        nl_seed: "Build an RWA share registry: owner-gated issuance up to totalSupply, allowlist-gated transfers, per-transaction cap, and a rolling block-window spending cap.".into(),
        ctor_sig: Some("constructor(uint64,uint64,uint64)".into()),
        ctor_hints: vec![
            "totalSupply".into(),
            "maxPerTx".into(),
            "windowCap".into(),
        ],
        tags: vec!["rwa".into(), "evm".into(), "xlayer".into()],
        design: Some("proofship-dapp".into()),
        source: Some(include_str!("../../../../proofship/templates/rwa-share-registry/program.lean").into()),
        abi_json: Some(
            include_str!("../../../../proofship/templates/rwa-share-registry/abi.json").into(),
        ),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_rwa_prefers_xlayer() {
        let store = TemplateStore::new(Vec::<PathBuf>::new());
        let list = store.list().unwrap();
        assert!(!list.is_empty());
        let rwa = store.get("rwa-share-registry").unwrap();
        assert_eq!(rwa.preferred_network_id, "xlayer-testnet");
        assert!(rwa.source.as_ref().unwrap().contains("import ProofForgeV2"));
        assert!(rwa.abi_json.as_ref().unwrap().contains("totalSupply"));
    }
}
