use dioxus::prelude::*;

use crate::app::{NetworkMode, Tab};

#[component]
pub fn NetworkToggleOption(
    network_mode: Signal<NetworkMode>,
    mode: NetworkMode,
    on_select: EventHandler<NetworkMode>,
) -> Element {
    let is_selected = *network_mode.read() == mode;
    let mut setter = network_mode;
    rsx! {
        label {
            title: format_args!(
                "Switch Pubky facade to the {} network, matching the client constructors in pubky::PubkyHttpClient",
                mode.label()
            ),
            input {
                r#type: "radio",
                name: "network-mode",
                checked: is_selected,
                title: format_args!(
                    "Use {} endpoints for all homeserver and HTTP calls",
                    mode.label()
                ),
                onchange: move |_| {
                    setter.set(mode);
                    on_select.call(mode);
                },
            }
            span { "{mode.label()}" }
        }
    }
}

#[component]
pub fn TabButton(tab: Tab, active_tab: Signal<Tab>) -> Element {
    let is_active = *active_tab.read() == tab;
    let mut setter = active_tab;
    let class_name = if is_active { "action active" } else { "action" };
    rsx! {
        button {
            class: class_name,
            title: format_args!(
                "Open the {} tools – a quick tour of the related pubky APIs",
                tab.label()
            ),
            onclick: move |_| setter.set(tab),
            "{tab.label()}"
        }
    }
}
