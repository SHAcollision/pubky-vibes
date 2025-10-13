use std::{
    env, fs,
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{Context, Result, anyhow};
use dioxus::prelude::WritableExt;
use dioxus::signals::{Signal, SignalData, Storage};
use directories::ProjectDirs;
use pubky_homeserver::{ConfigToml, Domain, LoggingToml, SignupMode};

/// Shape of the editable configuration exposed in the UI form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ConfigForm {
    pub(crate) signup_mode: SignupMode,
    pub(crate) drive_pubky_listen_socket: String,
    pub(crate) drive_icann_listen_socket: String,
    pub(crate) admin_listen_socket: String,
    pub(crate) admin_password: String,
    pub(crate) pkdns_public_ip: String,
    pub(crate) pkdns_public_pubky_tls_port: String,
    pub(crate) pkdns_public_icann_http_port: String,
    pub(crate) pkdns_icann_domain: String,
    pub(crate) logging_level: String,
}

impl ConfigForm {
    fn from_config(config: &ConfigToml) -> Self {
        Self {
            signup_mode: config.general.signup_mode.clone(),
            drive_pubky_listen_socket: config.drive.pubky_listen_socket.to_string(),
            drive_icann_listen_socket: config.drive.icann_listen_socket.to_string(),
            admin_listen_socket: config.admin.listen_socket.to_string(),
            admin_password: config.admin.admin_password.clone(),
            pkdns_public_ip: config.pkdns.public_ip.to_string(),
            pkdns_public_pubky_tls_port: config
                .pkdns
                .public_pubky_tls_port
                .map(|p| p.to_string())
                .unwrap_or_default(),
            pkdns_public_icann_http_port: config
                .pkdns
                .public_icann_http_port
                .map(|p| p.to_string())
                .unwrap_or_default(),
            pkdns_icann_domain: config
                .pkdns
                .icann_domain
                .as_ref()
                .map(|d| d.to_string())
                .unwrap_or_default(),
            logging_level: config
                .logging
                .as_ref()
                .map(|logging| logging.level.to_string())
                .unwrap_or_default(),
        }
    }

    pub(crate) fn default() -> Self {
        Self::from_config(&ConfigToml::default())
    }
}

/// Tracks the current form state, whether there are unsaved changes, and any
/// feedback to render to the operator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ConfigState {
    pub(crate) form: ConfigForm,
    pub(crate) dirty: bool,
    pub(crate) feedback: Option<ConfigFeedback>,
}

/// Feedback returned to the operator when saving or loading configuration data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ConfigFeedback {
    Saved,
    ValidationError(String),
    PersistenceError(String),
}

/// Outcome returned by [`persist_config_form`] indicating whether the TOML file was
/// rewritten.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConfigPersistOutcome {
    Updated,
    Unchanged,
}

pub(crate) fn load_config_form_from_dir(data_dir: &str) -> Result<ConfigForm> {
    if data_dir.trim().is_empty() {
        return Ok(ConfigForm::default());
    }

    let config_path = Path::new(data_dir).join("config.toml");
    if config_path.is_file() {
        let config = ConfigToml::from_file(&config_path)
            .map_err(|err| anyhow!("Failed to read {}: {}", config_path.display(), err))?;
        Ok(ConfigForm::from_config(&config))
    } else {
        Ok(ConfigForm::default())
    }
}

pub(crate) fn config_state_from_dir(data_dir: &str) -> ConfigState {
    match load_config_form_from_dir(data_dir) {
        Ok(form) => ConfigState {
            form,
            dirty: false,
            feedback: None,
        },
        Err(err) => ConfigState {
            form: ConfigForm::default(),
            dirty: false,
            feedback: Some(ConfigFeedback::PersistenceError(err.to_string())),
        },
    }
}

pub(crate) fn persist_config_form(
    data_dir: &str,
    form: &ConfigForm,
) -> Result<ConfigPersistOutcome> {
    let trimmed = data_dir.trim();
    if trimmed.is_empty() {
        return Err(anyhow!(
            "Please provide a directory where we can write config.toml."
        ));
    }

    let dir_path = PathBuf::from(trimmed);
    let config_path = dir_path.join("config.toml");

    let (mut config, had_existing) = if config_path.is_file() {
        let existing = ConfigToml::from_file(&config_path)
            .map_err(|err| anyhow!("Failed to parse {}: {}", config_path.display(), err))?;
        (existing, true)
    } else {
        (ConfigToml::default(), false)
    };

    let baseline = if had_existing {
        Some(config.clone())
    } else {
        None
    };

    apply_config_form(form, &mut config)?;

    if let Some(previous) = baseline
        && previous == config
    {
        return Ok(ConfigPersistOutcome::Unchanged);
    }

    fs::create_dir_all(&dir_path)
        .with_context(|| format!("Failed to create data directory at {}", dir_path.display()))?;

    let rendered =
        toml::to_string_pretty(&config).context("Failed to render config as TOML text")?;
    fs::write(&config_path, rendered)
        .with_context(|| format!("Failed to write {}", config_path.display()))?;

    Ok(ConfigPersistOutcome::Updated)
}

pub(crate) fn apply_config_form(form: &ConfigForm, config: &mut ConfigToml) -> Result<()> {
    config.general.signup_mode = form.signup_mode.clone();

    config.drive.pubky_listen_socket =
        parse_socket("Pubky TLS listen socket", &form.drive_pubky_listen_socket)?;
    config.drive.icann_listen_socket =
        parse_socket("ICANN HTTP listen socket", &form.drive_icann_listen_socket)?;

    config.admin.listen_socket = parse_socket("Admin listen socket", &form.admin_listen_socket)?;
    config.admin.admin_password = form.admin_password.clone();

    config.pkdns.public_ip = parse_ip("Public IP", &form.pkdns_public_ip)?;
    config.pkdns.public_pubky_tls_port =
        parse_optional_port("Public Pubky TLS port", &form.pkdns_public_pubky_tls_port)?;
    config.pkdns.public_icann_http_port =
        parse_optional_port("Public ICANN HTTP port", &form.pkdns_public_icann_http_port)?;
    config.pkdns.icann_domain = parse_optional_domain(&form.pkdns_icann_domain)?;

    let logging = parse_logging_level(&form.logging_level, config.logging.clone())?;
    config.logging = logging;

    Ok(())
}

pub(crate) fn modify_config_form<F, S>(mut state: Signal<ConfigState, S>, update: F)
where
    F: FnOnce(&mut ConfigForm),
    S: Storage<SignalData<ConfigState>> + 'static,
{
    let mut guard = state.write();
    update(&mut guard.form);
    guard.dirty = true;
    guard.feedback = None;
}

#[cfg(target_os = "android")]
pub(crate) fn default_data_dir() -> String {
    super::android_data_dir().to_string_lossy().into_owned()
}

#[cfg(not(target_os = "android"))]
pub(crate) fn default_data_dir() -> String {
    if let Some(project_dirs) = ProjectDirs::from("io", "Pubky", "PortableHomeserver") {
        project_dirs.data_dir().to_string_lossy().into_owned()
    } else {
        let mut fallback = env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        fallback.push(".pubky");
        fallback.to_string_lossy().into_owned()
    }
}

fn parse_socket(label: &str, raw: &str) -> Result<SocketAddr> {
    raw.trim()
        .parse()
        .map_err(|err| anyhow!("{} must be in host:port format ({}).", label, err))
}

fn parse_ip(label: &str, raw: &str) -> Result<IpAddr> {
    raw.trim()
        .parse()
        .map_err(|err| anyhow!("{} is not a valid IP address ({}).", label, err))
}

fn parse_optional_port(label: &str, raw: &str) -> Result<Option<u16>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    trimmed
        .parse()
        .map(Some)
        .map_err(|err| anyhow!("{} must be a port number ({}).", label, err))
}

fn parse_optional_domain(raw: &str) -> Result<Option<Domain>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    Domain::from_str(trimmed)
        .map(Some)
        .map_err(|err| anyhow!("Invalid domain '{}': {}", trimmed, err))
}

fn parse_logging_level(raw: &str, existing: Option<LoggingToml>) -> Result<Option<LoggingToml>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(existing.map(|mut logging| {
            logging.level = LoggingToml::default().level;
            logging
        }));
    }

    let parsed = trimmed.parse().map_err(|err| {
        anyhow!(
            "Invalid logging level '{}': {}. Use trace, debug, info, warn, or error.",
            trimmed,
            err
        )
    })?;

    let mut logging = existing.unwrap_or_default();
    logging.level = parsed;
    Ok(Some(logging))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;

    #[test]
    fn config_form_roundtrip_preserves_updates() {
        let mut config = ConfigToml::default();
        config.general.signup_mode = SignupMode::Open;
        config.admin.admin_password = "changed".into();
        config.pkdns.public_ip = "192.168.1.1".parse::<IpAddr>().unwrap();

        let form = ConfigForm::from_config(&config);
        let mut applied = ConfigToml::default();
        apply_config_form(&form, &mut applied).expect("form should apply cleanly");

        assert_eq!(applied.general.signup_mode, SignupMode::Open);
        assert_eq!(applied.admin.admin_password, "changed");
        assert_eq!(
            applied.pkdns.public_ip,
            "192.168.1.1".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn apply_config_form_rejects_invalid_port() {
        let mut form = ConfigForm::default();
        form.pkdns_public_pubky_tls_port = "invalid".into();

        let mut config = ConfigToml::default();
        let err = apply_config_form(&form, &mut config)
            .expect_err("invalid port should produce an error");

        assert!(err.to_string().contains("port"));
    }

    #[test]
    fn persist_config_form_writes_file() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let mut form = ConfigForm::default();
        form.admin_password = "super-secure".into();

        let outcome = persist_config_form(temp_dir.path().to_str().unwrap(), &form)
            .expect("config should persist");
        assert_eq!(outcome, ConfigPersistOutcome::Updated);

        let saved = ConfigToml::from_file(temp_dir.path().join("config.toml"))
            .expect("config should parse");
        assert_eq!(saved.admin.admin_password, "super-secure");
    }

    #[test]
    fn persist_config_form_detects_unchanged_input() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let form = ConfigForm::default();

        let first = persist_config_form(temp_dir.path().to_str().unwrap(), &form)
            .expect("initial write should succeed");
        assert_eq!(first, ConfigPersistOutcome::Updated);

        let second = persist_config_form(temp_dir.path().to_str().unwrap(), &form)
            .expect("second write should short circuit");
        assert_eq!(second, ConfigPersistOutcome::Unchanged);
    }
}
