use anyhow::Result;
use pubky::Pubky;

use crate::app::NetworkMode;

pub fn build_pubky(mode: NetworkMode) -> Result<Pubky> {
    match mode {
        NetworkMode::Mainnet => Ok(Pubky::new()?),
        NetworkMode::Testnet => Ok(Pubky::testnet()?),
    }
}
