use std::future::Future;

use anyhow::{Result, anyhow};
use dioxus::prelude::{Signal, WritableExt, spawn};
use reqwest::header::HeaderName;

use crate::{LogEntry, LogLevel, push_log};

#[derive(Clone, Copy)]
pub struct RequestMessages {
    pub success_prefix: &'static str,
    pub error_prefix: &'static str,
}

impl RequestMessages {
    pub const fn new(success_prefix: &'static str, error_prefix: &'static str) -> Self {
        Self {
            success_prefix,
            error_prefix,
        }
    }
}

pub struct RequestOutcome {
    pub response: Option<String>,
    pub message: String,
}

impl RequestOutcome {
    pub fn with_response(response: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            response: Some(response.into()),
            message: message.into(),
        }
    }
}

pub fn run_request<Fut>(
    logs: Signal<Vec<LogEntry>>,
    mut response: Signal<String>,
    future: Fut,
    messages: RequestMessages,
) where
    Fut: Future<Output = Result<RequestOutcome>> + 'static,
{
    spawn(async move {
        match future.await {
            Ok(outcome) => {
                if let Some(resp) = outcome.response {
                    response.set(resp);
                }
                let log_message = format!("{}{}", messages.success_prefix, outcome.message);
                push_log(logs, LogLevel::Success, log_message);
            }
            Err(err) => {
                let log_message = format!("{}: {err}", messages.error_prefix);
                push_log(logs, LogLevel::Error, log_message);
            }
        }
    });
}

pub fn parse_headers(input: &str) -> Result<Vec<(HeaderName, String)>> {
    let mut parsed = Vec::new();
    for (index, line) in input.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let (name, value) = trimmed
            .split_once(':')
            .ok_or_else(|| anyhow!("Header must use Name: Value format on line {}", index + 1))?;
        let header_name: HeaderName = name.trim().parse()?;
        parsed.push((header_name, value.trim().to_string()));
    }
    Ok(parsed)
}
