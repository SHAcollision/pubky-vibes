use anyhow::{Context, Result, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use pubky::{Keypair, recovery_file};
use std::fs;
use std::path::{Path, PathBuf};

pub fn decode_secret_key(value: &str) -> Result<Keypair> {
    let bytes = STANDARD
        .decode(value.trim())
        .context("secret key must be valid base64")?;
    let secret: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow!("secret key must be 32 bytes"))?;
    Ok(Keypair::from_secret_key(&secret))
}

pub fn load_keypair_from_recovery(path: impl AsRef<Path>, passphrase: &str) -> Result<Keypair> {
    let bytes = fs::read(path.as_ref())
        .with_context(|| format!("failed to read {}", path.as_ref().display()))?;
    let kp = recovery_file::decrypt_recovery_file(&bytes, passphrase)?;
    Ok(kp)
}

pub fn save_keypair_to_recovery_file(
    keypair: &Keypair,
    path: &str,
    passphrase: &str,
) -> Result<PathBuf> {
    let normalized = normalize_pkarr_path(path)?;
    if let Some(parent) = normalized.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }
    let bytes = recovery_file::create_recovery_file(keypair, passphrase);
    fs::write(&normalized, bytes)
        .with_context(|| format!("failed to write {}", normalized.display()))?;
    Ok(normalized)
}

pub fn normalize_pkarr_path(input: &str) -> Result<PathBuf> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("path cannot be empty"));
    }

    let mut expanded = if let Some(stripped) = trimmed.strip_prefix('~') {
        let home = resolve_home_dir().context("unable to resolve home directory")?;
        if stripped.starts_with('/') || stripped.starts_with('\\') {
            home.join(&stripped[1..])
        } else if stripped.is_empty() {
            home
        } else {
            home.join(stripped)
        }
    } else {
        PathBuf::from(trimmed)
    };

    let needs_extension = expanded
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| !ext.eq_ignore_ascii_case("pkarr"))
        .unwrap_or(true);
    if needs_extension {
        expanded.set_extension("pkarr");
    }

    Ok(expanded)
}

fn resolve_home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use base64::engine::general_purpose::STANDARD;
    use std::ffi::OsString;
    use std::path::Path;
    use tempfile::TempDir;

    struct EnvGuard {
        key: &'static str,
        original: Option<OsString>,
    }

    impl EnvGuard {
        fn set_path(key: &'static str, value: &Path) -> Self {
            let original = std::env::var_os(key);
            // `std::env::set_var` is `unsafe` on the 2024 edition surface while
            // the standard library finalises its strictly-checked contract, so
            // keep the unsafety contained to this helper.
            unsafe {
                std::env::set_var(key, value);
            }
            Self { key, original }
        }

        fn remove(key: &'static str) -> Self {
            let original = std::env::var_os(key);
            unsafe {
                std::env::remove_var(key);
            }
            Self { key, original }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(ref value) = self.original {
                unsafe {
                    std::env::set_var(self.key, value);
                }
            } else {
                unsafe {
                    std::env::remove_var(self.key);
                }
            }
        }
    }

    #[test]
    fn decode_secret_key_roundtrip() -> Result<()> {
        let secret = [0x42u8; 32];
        let encoded = STANDARD.encode(secret);
        let decoded = decode_secret_key(&encoded)?;
        assert_eq!(decoded.secret_key(), secret);
        Ok(())
    }

    #[test]
    fn decode_secret_key_rejects_invalid_base64() {
        let err = decode_secret_key("not-base64").unwrap_err();
        assert!(err.to_string().contains("base64"));
    }

    #[test]
    fn normalize_pkarr_path_adds_extension_and_expands_home() -> Result<()> {
        let home = TempDir::new()?;
        let _guard_home = EnvGuard::set_path("HOME", home.path());
        let _guard_profile = EnvGuard::remove("USERPROFILE");

        let normalized = normalize_pkarr_path("~/keys/my-key")?;
        assert!(normalized.starts_with(home.path()));
        assert_eq!(
            normalized.extension().and_then(|ext| ext.to_str()),
            Some("pkarr")
        );
        assert_eq!(
            normalized.file_name().and_then(|name| name.to_str()),
            Some("my-key.pkarr")
        );
        Ok(())
    }

    #[test]
    fn normalize_pkarr_path_keeps_existing_extension() -> Result<()> {
        let path = normalize_pkarr_path("/tmp/example.PKARR")?;
        assert_eq!(path.extension().and_then(|ext| ext.to_str()), Some("PKARR"));
        Ok(())
    }

    #[test]
    fn normalize_pkarr_path_rejects_empty_input() {
        let err = normalize_pkarr_path("").unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn save_and_load_keypair_through_recovery_file() -> Result<()> {
        let keypair = Keypair::from_secret_key(&[7u8; 32]);
        let dir = TempDir::new()?;
        let target = dir.path().join("nested/subdir/key");
        let target_str = target.to_string_lossy();
        let saved = save_keypair_to_recovery_file(&keypair, &target_str, "passphrase")?;
        assert!(saved.exists());
        assert_eq!(
            saved.extension().and_then(|ext| ext.to_str()),
            Some("pkarr")
        );

        let restored = load_keypair_from_recovery(&saved, "passphrase")?;
        assert_eq!(restored.secret_key(), keypair.secret_key());
        Ok(())
    }
}
