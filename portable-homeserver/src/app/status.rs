use super::state::{NetworkProfile, ServerInfo, ServerStatus};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct StatusCopy {
    pub(crate) class_name: &'static str,
    pub(crate) heading: &'static str,
    pub(crate) summary: &'static str,
}

pub(crate) fn status_copy(status: &ServerStatus) -> StatusCopy {
    match status {
        ServerStatus::Idle => StatusCopy {
            class_name: "idle",
            heading: "Homeserver is idle",
            summary: "Select a storage directory and click start to bring your node online.",
        },
        ServerStatus::Starting => StatusCopy {
            class_name: "starting",
            heading: "Starting homeserver",
            summary: "Loading configuration, generating keys, and opening network ports…",
        },
        ServerStatus::Running(info) => StatusCopy {
            class_name: "running",
            heading: "Homeserver is running",
            summary: match info.network {
                NetworkProfile::Mainnet => {
                    "Your Pubky agent is online and sharing data for your communities."
                }
                NetworkProfile::Testnet => {
                    "Static testnet services are online with fixed ports and credentials."
                }
            },
        },
        ServerStatus::Stopping => StatusCopy {
            class_name: "stopping",
            heading: "Stopping homeserver",
            summary: "Shutting down services and closing sockets…",
        },
        ServerStatus::Error(_) => StatusCopy {
            class_name: "error",
            heading: "Something went wrong",
            summary: "We couldn't boot the homeserver with the current settings.",
        },
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum StatusDetails {
    None,
    Message(&'static str),
    Error {
        message: String,
    },
    Running {
        network_label: String,
        network_hint: Option<&'static str>,
        admin_url: String,
        icann_url: String,
        pubky_url: String,
        public_key: String,
    },
}

pub(crate) fn status_details(status: &ServerStatus) -> StatusDetails {
    match status {
        ServerStatus::Idle => StatusDetails::None,
        ServerStatus::Starting => StatusDetails::Message(
            "This usually takes a few seconds – we wait for the admin and TLS endpoints to come online.",
        ),
        ServerStatus::Stopping => StatusDetails::Message(
            "Hold tight while we close the node. You can start it again once this completes.",
        ),
        ServerStatus::Error(message) => StatusDetails::Error {
            message: message.clone(),
        },
        ServerStatus::Running(info) => {
            let NetworkDisplay { label, hint } = network_display(info);
            StatusDetails::Running {
                network_label: label,
                network_hint: hint,
                admin_url: info.admin_url.clone(),
                icann_url: info.icann_http_url.clone(),
                pubky_url: info.pubky_url.clone(),
                public_key: info.public_key.clone(),
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct NetworkDisplay {
    label: String,
    hint: Option<&'static str>,
}

fn network_display(info: &ServerInfo) -> NetworkDisplay {
    let label = match info.network {
        NetworkProfile::Mainnet => info.network.label().to_string(),
        NetworkProfile::Testnet => {
            format!("{} · local relays & bootstrap", info.network.label())
        }
    };

    let hint = match info.network {
        NetworkProfile::Mainnet => None,
        NetworkProfile::Testnet => {
            Some("Static ports: DHT 6881, pkarr 15411, HTTP relay 15412, admin 6288.")
        }
    };

    NetworkDisplay { label, hint }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_info(network: NetworkProfile) -> ServerInfo {
        ServerInfo {
            public_key: "pk_test".into(),
            admin_url: "http://localhost:6288".into(),
            icann_http_url: "http://localhost:15412".into(),
            pubky_url: "https://example.pubky".into(),
            network,
        }
    }

    #[test]
    fn status_copy_reflects_idle_state() {
        let copy = status_copy(&ServerStatus::Idle);

        assert_eq!(
            copy,
            StatusCopy {
                class_name: "idle",
                heading: "Homeserver is idle",
                summary: "Select a storage directory and click start to bring your node online.",
            }
        );
    }

    #[test]
    fn status_copy_reflects_running_profiles() {
        let mainnet_copy =
            status_copy(&ServerStatus::Running(sample_info(NetworkProfile::Mainnet)));
        assert_eq!(
            mainnet_copy,
            StatusCopy {
                class_name: "running",
                heading: "Homeserver is running",
                summary: "Your Pubky agent is online and sharing data for your communities.",
            }
        );

        let testnet_copy =
            status_copy(&ServerStatus::Running(sample_info(NetworkProfile::Testnet)));
        assert_eq!(
            testnet_copy,
            StatusCopy {
                class_name: "running",
                heading: "Homeserver is running",
                summary: "Static testnet services are online with fixed ports and credentials.",
            }
        );
    }

    #[test]
    fn network_display_describes_profiles() {
        let mainnet = network_display(&sample_info(NetworkProfile::Mainnet));
        assert_eq!(mainnet.label, "Mainnet");
        assert_eq!(mainnet.hint, None);

        let testnet = network_display(&sample_info(NetworkProfile::Testnet));
        assert_eq!(testnet.label, "Static Testnet · local relays & bootstrap");
        assert_eq!(
            testnet.hint,
            Some("Static ports: DHT 6881, pkarr 15411, HTTP relay 15412, admin 6288.")
        );
    }

    #[test]
    fn status_details_returns_none_for_idle() {
        assert_eq!(status_details(&ServerStatus::Idle), StatusDetails::None);
    }

    #[test]
    fn status_details_returns_message_states() {
        assert_eq!(
            status_details(&ServerStatus::Starting),
            StatusDetails::Message(
                "This usually takes a few seconds – we wait for the admin and TLS endpoints to come online.",
            )
        );

        assert_eq!(
            status_details(&ServerStatus::Stopping),
            StatusDetails::Message(
                "Hold tight while we close the node. You can start it again once this completes.",
            )
        );
    }

    #[test]
    fn status_details_describes_errors() {
        let err = StatusDetails::Error {
            message: "boom".into(),
        };
        assert_eq!(status_details(&ServerStatus::Error("boom".into())), err);
    }

    #[test]
    fn status_details_summarises_running_info() {
        let info = sample_info(NetworkProfile::Testnet);
        let details = status_details(&ServerStatus::Running(info.clone()));

        assert_eq!(
            details,
            StatusDetails::Running {
                network_label: "Static Testnet · local relays & bootstrap".into(),
                network_hint: Some(
                    "Static ports: DHT 6881, pkarr 15411, HTTP relay 15412, admin 6288.",
                ),
                admin_url: info.admin_url,
                icann_url: info.icann_http_url,
                pubky_url: info.pubky_url,
                public_key: info.public_key,
            }
        );
    }
}
