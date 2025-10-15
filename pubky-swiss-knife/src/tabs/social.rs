use std::str::FromStr;

use anyhow::anyhow;
use dioxus::prelude::*;
use pubky_app_specs::{
    PubkyAppPost, PubkyAppPostEmbed, PubkyAppPostKind, PubkyAppTag, PubkyAppUser, PubkyAppUserLink,
    traits::{HasIdPath, HasPath, HashId, TimestampId, Validatable},
};
use serde_json::to_string_pretty;

use crate::tabs::SocialTabState;
use crate::utils::http::{format_response, format_response_parts};
use crate::utils::logging::ActivityLog;
use crate::utils::mobile::{is_android_touch, touch_copy_option, touch_tooltip};
use crate::utils::pubky::PubkyFacadeHandle;

#[allow(clippy::too_many_arguments, clippy::clone_on_copy)]
pub fn render_social_tab(
    _pubky: PubkyFacadeHandle,
    state: SocialTabState,
    logs: ActivityLog,
) -> Element {
    let SocialTabState {
        session,
        profile_name,
        profile_bio,
        profile_image,
        profile_status,
        profile_links,
        profile_error,
        profile_response,
        post_content,
        post_kind,
        post_parent,
        post_embed_kind,
        post_embed_uri,
        post_attachments,
        post_response,
        tag_uri,
        tag_label,
        tag_response,
    } = state;

    let has_session = session.read().is_some();

    let profile_name_value = profile_name.read().clone();
    let profile_bio_value = profile_bio.read().clone();
    let profile_image_value = profile_image.read().clone();
    let profile_status_value = profile_status.read().clone();
    let profile_links_value = profile_links.read().clone();
    let profile_error_value = profile_error.read().clone();
    let profile_response_value = profile_response.read().clone();

    let post_content_value = post_content.read().clone();
    let post_kind_value = post_kind.read().clone();
    let post_parent_value = post_parent.read().clone();
    let post_embed_kind_value = post_embed_kind.read().clone();
    let post_embed_uri_value = post_embed_uri.read().clone();
    let post_attachments_value = post_attachments.read().clone();
    let post_response_value = post_response.read().clone();

    let tag_uri_value = tag_uri.read().clone();
    let tag_label_value = tag_label.read().clone();
    let tag_response_value = tag_response.read().clone();

    let profile_copy_value = if profile_response_value.trim().is_empty() {
        None
    } else {
        Some(profile_response_value.clone())
    };
    let post_copy_value = if post_response_value.trim().is_empty() {
        None
    } else {
        Some(post_response_value.clone())
    };
    let tag_copy_value = if tag_response_value.trim().is_empty() {
        None
    } else {
        Some(tag_response_value.clone())
    };

    let copy_success = if is_android_touch() {
        Some(String::from("Copied response to clipboard"))
    } else {
        None
    };

    let profile_fetch_session = session.clone();
    let profile_fetch_logs = logs.clone();
    let profile_fetch_name = profile_name.clone();
    let profile_fetch_bio = profile_bio.clone();
    let profile_fetch_image = profile_image.clone();
    let profile_fetch_status = profile_status.clone();
    let profile_fetch_links = profile_links.clone();
    let profile_fetch_error = profile_error.clone();
    let profile_fetch_response = profile_response.clone();

    let profile_save_session = session.clone();
    let profile_save_logs = logs.clone();
    let profile_save_name = profile_name.clone();
    let profile_save_bio = profile_bio.clone();
    let profile_save_image = profile_image.clone();
    let profile_save_status = profile_status.clone();
    let profile_save_links = profile_links.clone();
    let mut profile_save_error = profile_error.clone();
    let profile_save_response = profile_response.clone();

    let post_create_session = session.clone();
    let post_create_logs = logs.clone();
    let post_create_content = post_content.clone();
    let post_create_kind = post_kind.clone();
    let post_create_parent = post_parent.clone();
    let post_create_embed_kind = post_embed_kind.clone();
    let post_create_embed_uri = post_embed_uri.clone();
    let post_create_attachments = post_attachments.clone();
    let post_create_response = post_response.clone();

    let tag_create_session = session.clone();
    let tag_create_logs = logs.clone();
    let tag_create_uri = tag_uri.clone();
    let tag_create_label = tag_label.clone();
    let tag_create_response = tag_response.clone();

    let mut profile_name_binding = profile_name.clone();
    let mut profile_bio_binding = profile_bio.clone();
    let mut profile_image_binding = profile_image.clone();
    let mut profile_status_binding = profile_status.clone();
    let mut profile_links_binding = profile_links.clone();

    let mut post_content_binding = post_content.clone();
    let mut post_kind_binding = post_kind.clone();
    let mut post_parent_binding = post_parent.clone();
    let mut post_embed_kind_binding = post_embed_kind.clone();
    let mut post_embed_uri_binding = post_embed_uri.clone();
    let mut post_attachments_binding = post_attachments.clone();

    let mut tag_uri_binding = tag_uri.clone();
    let mut tag_label_binding = tag_label.clone();

    rsx! {
        div { class: "tab-body",
            if !has_session {
                section { class: "card",
                    h2 { "Session required" }
                    p { class: "helper-text", "Load or create a session to manage pubky.app social data." }
                }
            } else {
                section { class: "card",
                    h2 { "Profile" }
                    p { class: "helper-text", "View and update the social profile stored at /pub/pubky.app/profile.json." }
                    div { class: "small-buttons",
                        button {
                            class: "action",
                            title: "Fetch the profile from session storage",
                            "data-touch-tooltip": touch_tooltip("Fetch the profile from session storage"),
                            onclick: move |_| {
                                if let Some(session) = profile_fetch_session.read().as_ref().cloned() {
                                    let mut response_signal = profile_fetch_response.clone();
                                    let mut error_signal = profile_fetch_error.clone();
                                    let mut name_signal = profile_fetch_name.clone();
                                    let mut bio_signal = profile_fetch_bio.clone();
                                    let mut image_signal = profile_fetch_image.clone();
                                    let mut status_signal = profile_fetch_status.clone();
                                    let mut links_signal = profile_fetch_links.clone();
                                    let logs_task = profile_fetch_logs.clone();
                                    spawn(async move {
                                        let result = async {
                                            let response = session
                                                .storage()
                                                .get(PubkyAppUser::create_path())
                                                .await?;
                                            let status = response.status();
                                            let version = response.version();
                                            let headers = response.headers().clone();
                                            let body = response.bytes().await?.to_vec();
                                            let formatted =
                                                format_response_parts(status, version, &headers, &body);
                                            let profile = <PubkyAppUser as Validatable>::try_from(&body, "")
                                                .map_err(|err| anyhow!(err))?;
                                            Ok::<_, anyhow::Error>((formatted, profile))
                                        };
                                        match result.await {
                                            Ok((formatted, profile)) => {
                                                name_signal.set(profile.name.clone());
                                                bio_signal.set(profile.bio.unwrap_or_default());
                                                image_signal.set(profile.image.unwrap_or_default());
                                                status_signal.set(profile.status.unwrap_or_default());
                                                links_signal.set(format_links(profile.links.as_deref()));
                                                error_signal.set(String::new());
                                                response_signal.set(formatted.clone());
                                                logs_task.success("Loaded pubky.app profile");
                                            }
                                            Err(err) => {
                                                error_signal.set(err.to_string());
                                                response_signal.set(String::new());
                                                logs_task.error(format!("Failed to load profile: {err}"));
                                            }
                                        }
                                    });
                                } else {
                                    profile_fetch_logs.error("No active session");
                                }
                            },
                            "Load profile",
                        }
                    }
                    if !profile_name_value.trim().is_empty() || !profile_bio_value.trim().is_empty() || !profile_image_value.trim().is_empty() || !profile_status_value.trim().is_empty() {
                        div { class: "profile-preview",
                            h3 { "{profile_name_value}" }
                            if !profile_status_value.trim().is_empty() {
                                p { class: "helper-text", "Status: {profile_status_value}" }
                            }
                            if !profile_bio_value.trim().is_empty() {
                                p { class: "helper-text", "Bio: {profile_bio_value}" }
                            }
                            if !profile_image_value.trim().is_empty() {
                                img {
                                    class: "avatar-preview",
                                    src: profile_image_value.clone(),
                                    alt: "Profile avatar",
                                }
                            }
                            if !profile_links_value.trim().is_empty() {
                                ul { class: "helper-text",
                                    for link in profile_links_value.lines().filter(|line| !line.trim().is_empty()) {
                                        li { "{link}" }
                                    }
                                }
                            }
                        }
                    }
                    if !profile_error_value.trim().is_empty() {
                        p { class: "helper-text", style: "color: var(--danger-600);", "{profile_error_value}" }
                    }
                    div { class: "form-grid",
                        label {
                            "Display name"
                            input {
                                value: profile_name_value.clone(),
                                oninput: move |evt| profile_name_binding.set(evt.value()),
                                title: "Public display name for your profile",
                                "data-touch-tooltip": touch_tooltip("Public display name for your profile"),
                            }
                        }
                        label {
                            "Bio"
                            textarea {
                                value: profile_bio_value.clone(),
                                oninput: move |evt| profile_bio_binding.set(evt.value()),
                                title: "Short biography shown on your profile",
                                "data-touch-tooltip": touch_tooltip("Short biography shown on your profile"),
                            }
                        }
                        label {
                            "Avatar URL"
                            input {
                                value: profile_image_value.clone(),
                                oninput: move |evt| profile_image_binding.set(evt.value()),
                                title: "HTTPS link to an avatar image",
                                "data-touch-tooltip": touch_tooltip("HTTPS link to an avatar image"),
                            }
                        }
                        label {
                            "Status message"
                            input {
                                value: profile_status_value.clone(),
                                oninput: move |evt| profile_status_binding.set(evt.value()),
                                title: "Optional short status text",
                                "data-touch-tooltip": touch_tooltip("Optional short status text"),
                            }
                        }
                        label {
                            "Links"
                            textarea {
                                class: "tall",
                                value: profile_links_value.clone(),
                                oninput: move |evt| profile_links_binding.set(evt.value()),
                                title: "One link per line as Title | https://example.com",
                                "data-touch-tooltip": touch_tooltip("One link per line as Title | https://example.com"),
                            }
                        }
                    }
                    div { class: "small-buttons",
                        button {
                            class: "action secondary",
                            title: "Save these fields to profile.json",
                            "data-touch-tooltip": touch_tooltip("Save these fields to profile.json"),
                            onclick: move |_| {
                                if let Some(session) = profile_save_session.read().as_ref().cloned() {
                                    let name = profile_save_name.read().clone();
                                    if name.trim().is_empty() {
                                        profile_save_logs.error("Provide a display name");
                                        profile_save_error.set(String::from("Display name is required"));
                                        return;
                                    }
                                    let bio_value = profile_save_bio.read().clone();
                                    let image_value = profile_save_image.read().clone();
                                    let status_value = profile_save_status.read().clone();
                                    let links_input = profile_save_links.read().clone();
                                    let bio = optional_field(&bio_value);
                                    let image = optional_field(&image_value);
                                    let status = optional_field(&status_value);
                                    let links = match parse_links(&links_input) {
                                        Ok(links) => links,
                                        Err(err) => {
                                            profile_save_error.set(err.clone());
                                            profile_save_logs.error(err);
                                            return;
                                        }
                                    };
                                    profile_save_error.set(String::new());
                                    let user = PubkyAppUser::new(name.clone(), bio, image, links, status);
                                    if let Err(err) = user.validate(None) {
                                        let message = format!("Invalid profile data: {err}");
                                        profile_save_error.set(message.clone());
                                        profile_save_logs.error(message);
                                        return;
                                    }
                                    let path = PubkyAppUser::create_path();
                                    let body = match to_string_pretty(&user) {
                                        Ok(body) => body,
                                        Err(err) => {
                                            let message = format!("Failed to serialize profile: {err}");
                                            profile_save_error.set(message.clone());
                                            profile_save_logs.error(message);
                                            return;
                                        }
                                    };
                                    let mut response_signal = profile_save_response.clone();
                                    let mut error_signal = profile_save_error.clone();
                                    let logs_task = profile_save_logs.clone();
                                    spawn(async move {
                                        let result = async {
                                            let response = session.storage().put(path.clone(), body.clone()).await?;
                                            let formatted = format_response(response).await?;
                                            Ok::<_, anyhow::Error>(formatted)
                                        };
                                        match result.await {
                                            Ok(formatted) => {
                                                response_signal.set(formatted.clone());
                                                error_signal.set(String::new());
                                                logs_task.success("Updated profile.json");
                                            }
                                            Err(err) => {
                                                error_signal.set(err.to_string());
                                                response_signal.set(String::new());
                                                logs_task.error(format!("Failed to save profile: {err}"));
                                            }
                                        }
                                    });
                                } else {
                                    profile_save_logs.error("No active session");
                                }
                            },
                            "Save profile",
                        }
                    }
                    label {
                        "Latest response"
                        textarea {
                            readonly: true,
                            class: "log-output",
                            value: profile_response_value.clone(),
                            "data-touch-copy": touch_copy_option(profile_copy_value.clone()),
                            "data-touch-copy-success": copy_success.clone(),
                        }
                    }
                }

                section { class: "card",
                    h2 { "Posts" }
                    p { class: "helper-text", "Compose a new post for pubky.app feeds." }
                    div { class: "form-grid",
                        label {
                            "Content"
                            textarea {
                                class: "tall",
                                value: post_content_value.clone(),
                                oninput: move |evt| post_content_binding.set(evt.value()),
                                title: "Post body",
                                "data-touch-tooltip": touch_tooltip("Post body"),
                            }
                        }
                        label {
                            "Kind"
                            select {
                                value: post_kind_value.clone(),
                                oninput: move |evt| post_kind_binding.set(evt.value()),
                                title: "Select the type of post",
                                "data-touch-tooltip": touch_tooltip("Select the type of post"),
                                option { value: "short", "Short" }
                                option { value: "long", "Long" }
                                option { value: "image", "Image" }
                                option { value: "video", "Video" }
                                option { value: "link", "Link" }
                                option { value: "file", "File" }
                            }
                        }
                        label {
                            "Parent post URI"
                            input {
                                value: post_parent_value.clone(),
                                oninput: move |evt| post_parent_binding.set(evt.value()),
                                title: "Optional pubky:// URI of the parent post",
                                "data-touch-tooltip": touch_tooltip("Optional pubky:// URI of the parent post"),
                            }
                        }
                        label {
                            "Embed kind"
                            select {
                                value: post_embed_kind_value.clone(),
                                oninput: move |evt| post_embed_kind_binding.set(evt.value()),
                                title: "Type of embedded attachment",
                                "data-touch-tooltip": touch_tooltip("Type of embedded attachment"),
                                option { value: "", "None" }
                                option { value: "short", "Short" }
                                option { value: "long", "Long" }
                                option { value: "image", "Image" }
                                option { value: "video", "Video" }
                                option { value: "link", "Link" }
                                option { value: "file", "File" }
                            }
                        }
                        label {
                            "Embed URI"
                            input {
                                value: post_embed_uri_value.clone(),
                                oninput: move |evt| post_embed_uri_binding.set(evt.value()),
                                title: "URI to embed in the post",
                                "data-touch-tooltip": touch_tooltip("URI to embed in the post"),
                            }
                        }
                        label {
                            "Attachments"
                            textarea {
                                class: "tall",
                                value: post_attachments_value.clone(),
                                oninput: move |evt| post_attachments_binding.set(evt.value()),
                                title: "One attachment URI per line",
                                "data-touch-tooltip": touch_tooltip("One attachment URI per line"),
                            }
                        }
                    }
                    div { class: "small-buttons",
                        button {
                            class: "action secondary",
                            title: "Publish a new post",
                            "data-touch-tooltip": touch_tooltip("Publish a new post"),
                            onclick: move |_| {
                                if let Some(session) = post_create_session.read().as_ref().cloned() {
                                    let content = post_create_content.read().clone();
                                    if content.trim().is_empty() {
                                        post_create_logs.error("Post content cannot be empty");
                                        return;
                                    }
                                    let kind_value = post_create_kind.read().clone();
                                    let kind = match parse_post_kind(&kind_value) {
                                        Ok(kind) => kind,
                                        Err(err) => {
                                            post_create_logs.error(err);
                                            return;
                                        }
                                    };
                                    let parent_value = post_create_parent.read().clone();
                                    let embed_kind_str = post_create_embed_kind.read().clone();
                                    let embed_uri_str = post_create_embed_uri.read().clone();
                                    let embed = match parse_embed(&embed_kind_str, &embed_uri_str) {
                                        Ok(embed) => embed,
                                        Err(err) => {
                                            post_create_logs.error(err);
                                            return;
                                        }
                                    };
                                    let parent = optional_field(&parent_value);
                                    let attachments_value = post_create_attachments.read().clone();
                                    let attachments = parse_attachments(&attachments_value);
                                    let post = PubkyAppPost::new(content.clone(), kind, parent, embed, attachments);
                                    let post_id = post.create_id();
                                    if let Err(err) = post.validate(Some(&post_id)) {
                                        post_create_logs.error(format!("Invalid post: {err}"));
                                        return;
                                    }
                                    let path = PubkyAppPost::create_path(&post_id);
                                    let body = match to_string_pretty(&post) {
                                        Ok(body) => body,
                                        Err(err) => {
                                            post_create_logs.error(format!("Failed to serialize post: {err}"));
                                            return;
                                        }
                                    };
                                    let mut response_signal = post_create_response.clone();
                                    let logs_task = post_create_logs.clone();
                                    spawn(async move {
                                        let result = async {
                                            let response = session.storage().put(path.clone(), body.clone()).await?;
                                            let formatted = format_response(response).await?;
                                            Ok::<_, anyhow::Error>((formatted, path.clone()))
                                        };
                                        match result.await {
                                            Ok((formatted, path)) => {
                                                response_signal.set(formatted.clone());
                                                logs_task.success(format!("Published post to {path}"));
                                            }
                                            Err(err) => {
                                                response_signal.set(String::new());
                                                logs_task.error(format!("Failed to publish post: {err}"));
                                            }
                                        }
                                    });
                                } else {
                                    post_create_logs.error("No active session");
                                }
                            },
                            "Publish post",
                        }
                    }
                    label {
                        "Latest response"
                        textarea {
                            readonly: true,
                            class: "log-output",
                            value: post_response_value.clone(),
                            "data-touch-copy": touch_copy_option(post_copy_value.clone()),
                            "data-touch-copy-success": copy_success.clone(),
                        }
                    }
                }

                section { class: "card",
                    h2 { "Tags" }
                    p { class: "helper-text", "Attach a tag to an existing social URI." }
                    div { class: "form-grid",
                        label {
                            "Target URI"
                            input {
                                value: tag_uri_value.clone(),
                                oninput: move |evt| tag_uri_binding.set(evt.value()),
                                title: "pubky:// URI to tag",
                                "data-touch-tooltip": touch_tooltip("pubky:// URI to tag"),
                            }
                        }
                        label {
                            "Label"
                            input {
                                value: tag_label_value.clone(),
                                oninput: move |evt| tag_label_binding.set(evt.value()),
                                title: "Short lowercase tag label",
                                "data-touch-tooltip": touch_tooltip("Short lowercase tag label"),
                            }
                        }
                    }
                    div { class: "small-buttons",
                        button {
                            class: "action secondary",
                            title: "Create a tag",
                            "data-touch-tooltip": touch_tooltip("Create a tag"),
                            onclick: move |_| {
                                if let Some(session) = tag_create_session.read().as_ref().cloned() {
                                    let uri = tag_create_uri.read().clone();
                                    if uri.trim().is_empty() {
                                        tag_create_logs.error("Provide a URI to tag");
                                        return;
                                    }
                                    let label = tag_create_label.read().clone();
                                    if label.trim().is_empty() {
                                        tag_create_logs.error("Provide a tag label");
                                        return;
                                    }
                                    let tag = PubkyAppTag::new(uri.clone(), label.clone());
                                    let tag_id = tag.create_id();
                                    if let Err(err) = tag.validate(Some(&tag_id)) {
                                        tag_create_logs.error(format!("Invalid tag: {err}"));
                                        return;
                                    }
                                    let path = PubkyAppTag::create_path(&tag_id);
                                    let body = match to_string_pretty(&tag) {
                                        Ok(body) => body,
                                        Err(err) => {
                                            tag_create_logs.error(format!("Failed to serialize tag: {err}"));
                                            return;
                                        }
                                    };
                                    let mut response_signal = tag_create_response.clone();
                                    let logs_task = tag_create_logs.clone();
                                    spawn(async move {
                                        let result = async {
                                            let response = session.storage().put(path.clone(), body.clone()).await?;
                                            let formatted = format_response(response).await?;
                                            Ok::<_, anyhow::Error>((formatted, path.clone()))
                                        };
                                        match result.await {
                                            Ok((formatted, path)) => {
                                                response_signal.set(formatted.clone());
                                                logs_task.success(format!("Created tag at {path}"));
                                            }
                                            Err(err) => {
                                                response_signal.set(String::new());
                                                logs_task.error(format!("Failed to create tag: {err}"));
                                            }
                                        }
                                    });
                                } else {
                                    tag_create_logs.error("No active session");
                                }
                            },
                            "Create tag",
                        }
                    }
                    label {
                        "Latest response"
                        textarea {
                            readonly: true,
                            class: "log-output",
                            value: tag_response_value.clone(),
                            "data-touch-copy": touch_copy_option(tag_copy_value.clone()),
                            "data-touch-copy-success": copy_success,
                        }
                    }
                }
            }
        }
    }
}

fn optional_field(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_links(input: &str) -> Result<Option<Vec<PubkyAppUserLink>>, String> {
    let mut links = Vec::new();
    for (idx, line) in input.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, '|');
        let title = parts
            .next()
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .ok_or_else(|| format!("Link {} is missing a title", idx + 1))?;
        let url = parts
            .next()
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .ok_or_else(|| format!("Link {} is missing a URL", idx + 1))?;
        links.push(PubkyAppUserLink {
            title: title.to_string(),
            url: url.to_string(),
        });
    }
    if links.is_empty() {
        Ok(None)
    } else {
        Ok(Some(links))
    }
}

fn parse_post_kind(value: &str) -> Result<PubkyAppPostKind, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(String::from("Select a post kind"));
    }
    PubkyAppPostKind::from_str(trimmed).map_err(|err| format!("Invalid post kind: {err}"))
}

fn parse_embed(kind: &str, uri: &str) -> Result<Option<PubkyAppPostEmbed>, String> {
    let kind = kind.trim();
    let uri = uri.trim();
    if kind.is_empty() && uri.is_empty() {
        return Ok(None);
    }
    if kind.is_empty() || uri.is_empty() {
        return Err(String::from("Provide both an embed kind and URI"));
    }
    let embed_kind =
        PubkyAppPostKind::from_str(kind).map_err(|err| format!("Invalid embed kind: {err}"))?;
    Ok(Some(PubkyAppPostEmbed {
        kind: embed_kind,
        uri: uri.to_string(),
    }))
}

fn parse_attachments(input: &str) -> Option<Vec<String>> {
    let attachments: Vec<String> = input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect();
    if attachments.is_empty() {
        None
    } else {
        Some(attachments)
    }
}

fn format_links(links: Option<&[PubkyAppUserLink]>) -> String {
    match links {
        Some(links) if !links.is_empty() => links
            .iter()
            .map(|link| format!("{} | {}", link.title, link.url))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}
