use dioxus::prelude::*;

use crate::services::LogEntry;

#[component]
#[allow(non_snake_case)]
pub fn ActivityLog(logs: Signal<Vec<LogEntry>>) -> Element {
    let entries = { logs.read().clone() };
    rsx! {
        div { class: "panel",
            h2 { "Activity" }
            if entries.is_empty() {
                p { "No activity yet." }
            } else {
                div { class: "log-feed",
                    for entry in entries {
                        div { class: format!("log-line {}", entry.level.css_class()),
                            span { class: "ts", "{entry.timestamp}" }
                            span { "{entry.message}" }
                        }
                    }
                }
            }
        }
    }
}
