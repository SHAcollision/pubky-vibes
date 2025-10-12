use dioxus::prelude::*;

use crate::app::{NetworkMode, Tab};
use crate::utils::mobile::touch_tooltip;

#[component]
pub fn NetworkToggleOption(
    network_mode: Signal<NetworkMode>,
    mode: NetworkMode,
    on_select: EventHandler<NetworkMode>,
) -> Element {
    let is_selected = *network_mode.read() == mode;
    let mut setter = network_mode;
    let label_tooltip = format!(
        "Switch the Swiss Knife to the {} network so every tool talks to the right homeserver",
        mode.label()
    );
    let radio_tooltip = format!(
        "Use {} endpoints for every homeserver and HTTP request",
        mode.label()
    );
    rsx! {
        label {
            class: "network-toggle-option",
            title: label_tooltip.clone(),
            data-touch-tooltip: touch_tooltip(label_tooltip),
            input {
                r#type: "radio",
                name: "network-mode",
                checked: is_selected,
                title: radio_tooltip.clone(),
                data-touch-tooltip: touch_tooltip(radio_tooltip),
                onchange: move |_| {
                    setter.set(mode);
                    on_select.call(mode);
                },
            }
            span { class: "network-toggle-text", "{mode.label()}" }
        }
    }
}

#[component]
pub fn TabButton(tab: Tab, active_tab: Signal<Tab>) -> Element {
    let is_active = *active_tab.read() == tab;
    let mut setter = active_tab;
    let class_name = if is_active { "action active" } else { "action" };
    let tab_label = tab.label();
    let (view_box, paths) = tab.icon();
    let tab_tooltip = format!(
        "Show the {} toolbox for exploring that part of Pubky",
        tab.label()
    );
    rsx! {
        button {
            class: class_name,
            aria_label: tab_label,
            title: tab_tooltip.clone(),
            data-touch-tooltip: touch_tooltip(tab_tooltip),
            onclick: move |_| setter.set(tab),
            span { class: "tab-icon", aria_hidden: "true",
                svg {
                    view_box: view_box,
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "1.5",
                    for path in paths {
                        path {
                            d: *path,
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                        }
                    }
                }
            }
            span { class: "tab-label", "{tab_label}" }
        }
    }
}
