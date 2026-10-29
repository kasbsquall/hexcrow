// SPDX-License-Identifier: MIT
//
// Test personas. Each party to a Hexcrow invoice acts through keys it uses for that invoice only,
// so nothing on-chain ties the invoice to a long-lived account. The secrets live in a local file
// that is git-ignored; they are testnet keys and never leave this machine.

use std::{collections::BTreeMap, fs, path::Path};

use anyhow::{Context, Result, bail};
use ootle_rs::{Network, key_provider::PrivateKeyProvider, keys::OotleSecretKey};
use serde::{Deserialize, Serialize};
use tari_crypto::ristretto::RistrettoSecretKey;
use tari_utilities::{ByteArray, hex::Hex};

pub const NETWORK: Network = Network::Esmeralda;

/// `publisher` pays for the template, `client` and `contractor` are the two parties, and `escrow`
/// is the per-invoice viewing key both parties hold: it decrypts the escrow output but has no
/// spending power, because the output is gated by its condition tree only.
pub const NAMES: [&str; 5] = ["publisher", "client", "contractor", "escrow", "stranger"];

#[derive(Serialize, Deserialize)]
struct StoredKey {
    account_secret: String,
    view_secret: String,
}

pub struct Personas {
    keys: BTreeMap<String, OotleSecretKey>,
}

impl Personas {
    pub fn load_or_create(path: &Path) -> Result<Self> {
        let mut stored: BTreeMap<String, StoredKey> = if path.exists() {
            serde_json::from_str(&fs::read_to_string(path)?).context("parsing personas file")?
        } else {
            BTreeMap::new()
        };
        let mut changed = false;
        for name in NAMES {
            if !stored.contains_key(name) {
                let key = OotleSecretKey::random(NETWORK);
                stored.insert(name.to_string(), StoredKey {
                    account_secret: key.account_secret().to_hex(),
                    view_secret: key.view_only_secret().to_hex(),
                });
                changed = true;
            }
        }
        if changed {
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir)?;
            }
            fs::write(path, serde_json::to_string_pretty(&stored)?)?;
        }
        let mut keys = BTreeMap::new();
        for (name, s) in stored {
            let account = RistrettoSecretKey::from_hex(&s.account_secret)
                .map_err(|e| anyhow::anyhow!("bad account secret for {name}: {e}"))?;
            let view = RistrettoSecretKey::from_hex(&s.view_secret)
                .map_err(|e| anyhow::anyhow!("bad view secret for {name}: {e}"))?;
            keys.insert(name, OotleSecretKey::new(NETWORK, account, view));
        }
        Ok(Self { keys })
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
