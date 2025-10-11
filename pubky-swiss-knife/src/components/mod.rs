use dioxus::prelude::*;

use crate::app::{NetworkMode, Tab};

#[component]
pub fn NetworkToggleOption(network_mode: Signal<NetworkMode>, mode: NetworkMode) -> Element {
    let is_selected = *network_mode.read() == mode;
    let mut setter = network_mode;
    rsx! {
        label {
            input {
                r#type: "radio",
                name: "network-mode",
                checked: is_selected,
                onchange: move |_| setter.set(mode),
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
            onclick: move |_| setter.set(tab),
            "{tab.label()}"
        }
    }
}
