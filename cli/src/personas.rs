// SPDX-License-Identifier: MIT
//
// Test personas. Each party to a Hexcrow invoice acts through keys it uses for that invoice only,
// so nothing on-chain ties the invoice to a long-lived account. The secrets live in a local file
// that is git-ignored; they are testnet keys and never leave this machine.

use std::{collections::BTreeMap, fs, path::{Path, PathBuf}};

use anyhow::{Context, Result, bail};
use ootle_rs::{Network, key_provider::PrivateKeyProvider, keys::OotleSecretKey};
use serde::{Deserialize, Serialize};
use tari_crypto::ristretto::RistrettoSecretKey;
use tari_utilities::{ByteArray, hex::Hex};

pub const NETWORK: Network = Network::Esmeralda;

/// Long-lived personas: `publisher` pays for the template and the book, `stranger` attacks.
/// Every invoice gets its own `client-N`, `contractor-N` and `escrow-N` keys (see `invoice_names`),
/// so nothing on-chain links one invoice to another or to a long-lived account.
pub const NAMES: [&str; 2] = ["publisher", "stranger"];

/// The one-time personas of invoice `n`: client, contractor, and the escrow viewing key both
/// parties hold. The escrow key decrypts that invoice's escrow output and nothing else; it has no
/// spending power, because the output is gated by its condition tree only.
pub fn invoice_names(n: u64) -> [String; 3] {
    [format!("client-{n}"), format!("contractor-{n}"), format!("escrow-{n}")]
}

#[derive(Serialize, Deserialize)]
struct StoredKey {
    account_secret: String,
    view_secret: String,
}

pub struct Personas {
    path: PathBuf,
    keys: BTreeMap<String, OotleSecretKey>,
}

impl Personas {
    pub fn load_or_create(path: &Path) -> Result<Self> {
        let stored: BTreeMap<String, StoredKey> = if path.exists() {
            serde_json::from_str(&fs::read_to_string(path)?).context("parsing personas file")?
        } else {
            BTreeMap::new()
        };
        let mut keys = BTreeMap::new();
        for (name, s) in stored {
            let account = RistrettoSecretKey::from_hex(&s.account_secret)
                .map_err(|e| anyhow::anyhow!("bad account secret for {name}: {e}"))?;
            let view = RistrettoSecretKey::from_hex(&s.view_secret)
                .map_err(|e| anyhow::anyhow!("bad view secret for {name}: {e}"))?;
            keys.insert(name, OotleSecretKey::new(NETWORK, account, view));
        }
        let mut personas = Self { path: path.to_path_buf(), keys };
        for name in NAMES {
            personas.ensure(name)?;
        }
        Ok(personas)
    }

    /// Creates a fresh key for `name` (from the OS random source) if it has none, and saves it.
    pub fn ensure(&mut self, name: &str) -> Result<()> {
        if self.keys.contains_key(name) {
            return Ok(());
        }
        self.keys.insert(name.to_string(), OotleSecretKey::random(NETWORK));
        let stored: BTreeMap<&String, StoredKey> = self
            .keys
            .iter()
            .map(|(n, k)| (n, StoredKey {
                account_secret: k.account_secret().to_hex(),
                view_secret: k.view_only_secret().to_hex(),
            }))
            .collect();
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        // Write then rename, so an interrupted write never leaves the only copy of the keys half done.
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_string_pretty(&stored)?)?;
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    pub fn secret(&self, name: &str) -> Result<&OotleSecretKey> {
        match self.keys.get(name) {
            Some(k) => Ok(k),
            None => bail!("unknown persona {name}"),
        }
    }

    pub fn provider(&self, name: &str) -> Result<PrivateKeyProvider> {
        Ok(PrivateKeyProvider::new(self.secret(name)?.clone()))
    }

    pub fn view_secret_hex(&self, name: &str) -> Result<String> {
        Ok(self.secret(name)?.view_only_secret().to_hex())
    }
}
