use super::tables::{Tables, TABLES_COUNT};
use heed::{Env, EnvOpenOptions};
use std::env;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use super::migrations;

pub const DEFAULT_MAP_SIZE: usize = 10995116277760; // 10TB (not = disk-space used)
const MAP_SIZE_ENV_VAR: &str = "PUBKY_LMDB_MAP_SIZE_BYTES";
const MIN_MAP_SIZE: usize = 16 * 1024 * 1024; // 16 MiB

fn desired_map_size() -> usize {
    let raw = match env::var(MAP_SIZE_ENV_VAR) {
        Ok(value) => value,
        Err(_) => return DEFAULT_MAP_SIZE,
    };

    let trimmed = raw.trim();
    if trimmed.is_empty() {
        tracing::warn!(
            "{MAP_SIZE_ENV_VAR} provided but empty; falling back to default LMDB map size"
        );
        return DEFAULT_MAP_SIZE;
    }

    let parsed = match trimmed.parse::<u64>() {
        Ok(value) => value,
        Err(error) => {
            tracing::warn!(
                ?error,
                value = trimmed,
                "Failed to parse {MAP_SIZE_ENV_VAR} as bytes; falling back to default"
            );
            return DEFAULT_MAP_SIZE;
        }
    };

    let coerced = if parsed == 0 {
        tracing::warn!(
            "{MAP_SIZE_ENV_VAR} resolved to zero; using minimum supported LMDB map size"
        );
        MIN_MAP_SIZE
    } else if parsed > usize::MAX as u64 {
        tracing::warn!(
            value = parsed,
            "{MAP_SIZE_ENV_VAR} exceeds platform usize; saturating to usize::MAX"
        );
        usize::MAX
    } else {
        parsed as usize
    };

    if coerced < MIN_MAP_SIZE {
        tracing::warn!(
            requested = coerced,
            minimum = MIN_MAP_SIZE,
            "{MAP_SIZE_ENV_VAR} below minimum supported LMDB map size; clamping"
        );
        return MIN_MAP_SIZE;
    }

    tracing::info!(
        value_bytes = coerced,
        "Using LMDB map size override from {MAP_SIZE_ENV_VAR}"
    );
    coerced
}

#[derive(Debug, Clone)]
pub struct LmDB {
    pub(crate) env: Env,
    pub(crate) tables: Tables,
    // Only used for testing purposes to keep the testdir alive.
    #[allow(dead_code)]
    test_dir: Option<Arc<tempfile::TempDir>>,
}

impl LmDB {
    /// # Safety
    /// DB uses LMDB, [opening][heed::EnvOpenOptions::open] which is marked unsafe,
    /// because the possible Undefined Behavior (UB) if the lock file is broken.
    pub unsafe fn open(main_dir: &Path) -> anyhow::Result<Self> {
        fs::create_dir_all(main_dir)?;

        let map_size = desired_map_size();

        let env = unsafe {
            EnvOpenOptions::new()
                .max_dbs(TABLES_COUNT)
                .map_size(map_size)
                .open(main_dir)
        }?;

        migrations::run(&env)?;
        let mut wtxn = env.write_txn()?;
        let tables = Tables::new(&env, &mut wtxn)?;
        wtxn.commit()?;

        let db = LmDB {
            env,
            tables,
            test_dir: None,
        };

        Ok(db)
    }

    // Create an ephemeral database for testing purposes.
    #[cfg(test)]
    pub fn test() -> LmDB {
        // Create a temporary directory for the test.
        let temp_dir = tempfile::tempdir().unwrap();
        let mut lmdb = unsafe { LmDB::open(temp_dir.path()).unwrap() };
        lmdb.test_dir = Some(Arc::new(temp_dir)); // Keep the directory alive for the duration of the test. As soon as all LmDB instances are dropped, the directory will be deleted automatically.

        lmdb
    }
}
