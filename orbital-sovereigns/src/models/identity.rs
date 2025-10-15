use std::fmt;

use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use chacha20poly1305::aead::{Aead, KeyInit, OsRng};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use pubky::{Keypair, PublicKey};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommanderProfile {
    pub label: String,
    pub public_key: String,
    pub homeserver: Option<String>,
}

impl CommanderProfile {
    pub fn new(
        label: impl Into<String>,
        public_key: PublicKey,
        homeserver: Option<String>,
    ) -> Self {
        Self {
            label: label.into(),
            public_key: public_key.to_string(),
            homeserver,
        }
    }
}

#[derive(Clone)]
pub struct CommanderIdentity {
    label: String,
    keypair: Option<Keypair>,
    recovery_hint: Option<String>,
    last_rotated: Option<OffsetDateTime>,
}

impl Default for CommanderIdentity {
    fn default() -> Self {
        Self {
            label: String::from("Unnamed Commander"),
            keypair: None,
            recovery_hint: None,
            last_rotated: None,
        }
    }
}

impl CommanderIdentity {
    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn set_label(&mut self, label: impl Into<String>) {
        self.label = label.into();
    }

    pub fn keypair(&self) -> Option<&Keypair> {
        self.keypair.as_ref()
    }

    pub fn set_keypair(&mut self, keypair: Keypair) {
        self.recovery_hint = Some(STANDARD.encode(keypair.secret_key()));
        self.last_rotated = Some(OffsetDateTime::now_utc());
        self.keypair = Some(keypair);
    }

    pub fn clear_keypair(&mut self) {
        self.keypair = None;
    }

    pub fn recovery_hint(&self) -> Option<&str> {
        self.recovery_hint.as_deref()
    }

    pub fn last_rotated(&self) -> Option<OffsetDateTime> {
        self.last_rotated
    }

    pub fn generate(&mut self) -> Keypair {
        let kp = Keypair::random();
        self.set_keypair(kp.clone());
        kp
    }

    pub fn import_secret_key(&mut self, secret_base64: &str) -> Result<Keypair> {
        let bytes = STANDARD
            .decode(secret_base64.trim())
            .context("Secret key must be valid base64")?;
        if bytes.len() != 32 {
            anyhow::bail!("Secret key must contain exactly 32 bytes");
        }
        let mut secret = [0u8; 32];
        secret.copy_from_slice(&bytes);
        let kp = Keypair::from_secret_key(&secret);
        self.set_keypair(kp.clone());
        Ok(kp)
    }

    pub fn public_profile(&self, homeserver: Option<String>) -> Option<CommanderProfile> {
        self.keypair
            .as_ref()
            .map(|kp| CommanderProfile::new(self.label.clone(), kp.public_key(), homeserver))
    }

    pub fn export_secret_key(&self) -> Option<String> {
        self.keypair
            .as_ref()
            .map(|kp| STANDARD.encode(kp.secret_key()))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VaultEnvelope {
    pub cipher_text: String,
    pub nonce: String,
}

impl VaultEnvelope {
    pub fn seal(key_material: &[u8; 32], payload: &[u8]) -> Self {
        let cipher = ChaCha20Poly1305::new(Key::from_slice(key_material));
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let cipher_text = cipher.encrypt(nonce, payload).expect("encryption failure");
        Self {
            cipher_text: STANDARD.encode(cipher_text),
            nonce: STANDARD.encode(nonce_bytes),
        }
    }

    pub fn open(&self, key_material: &[u8; 32]) -> Result<Vec<u8>> {
        let cipher = ChaCha20Poly1305::new(Key::from_slice(key_material));
        let nonce_bytes = STANDARD
            .decode(self.nonce.as_bytes())
            .context("Vault nonce must be valid base64")?;
        let cipher_bytes = STANDARD
            .decode(self.cipher_text.as_bytes())
            .context("Vault ciphertext must be valid base64")?;
        let nonce = Nonce::from_slice(&nonce_bytes);
        let plain = cipher
            .decrypt(nonce, cipher_bytes.as_ref())
            .context("Failed to decrypt vault envelope")?;
        Ok(plain)
    }
}

impl fmt::Debug for CommanderIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommanderIdentity")
            .field("label", &self.label)
            .field(
                "public_key",
                &self
                    .keypair
                    .as_ref()
                    .map(|kp| kp.public_key().to_string())
                    .unwrap_or_else(|| "(none)".into()),
            )
            .field("recovery_hint", &self.recovery_hint)
            .field("last_rotated", &self.last_rotated)
            .finish()
    }
}
