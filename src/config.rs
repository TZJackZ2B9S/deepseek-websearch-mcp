use std::{env, num::NonZeroU64, time::Duration};

use thiserror::Error;
use url::Url;

pub const DEFAULT_BASE_URL: &str = "https://api.deepseek.com/anthropic";
pub const DEFAULT_MODEL: &str = "deepseek-v4-flash";
pub const DEFAULT_TIMEOUT_SECS: u64 = 90;

#[derive(Clone)]
pub struct ApiKey(String);

impl ApiKey {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

#[derive(Clone)]
pub struct Config {
    pub api_key: Option<ApiKey>,
    pub base_url: Url,
    pub model: String,
    pub timeout: Duration,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let api_key = env::var("DEEPSEEK_API_KEY")
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .map(ApiKey::new);

        let base_url =
            env::var("DEEPSEEK_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_owned());
        let base_url = normalize_base_url(&base_url)?;

        let model = env::var("DEEPSEEK_MODEL")
            .unwrap_or_else(|_| DEFAULT_MODEL.to_owned())
            .trim()
            .to_owned();
        if model.is_empty() {
            return Err(ConfigError::EmptyModel);
        }

        let timeout = match env::var("DEEPSEEK_TIMEOUT_SECS") {
            Ok(raw) => {
                let seconds = raw
                    .parse::<NonZeroU64>()
                    .map_err(|_| ConfigError::InvalidTimeout(raw))?;
                Duration::from_secs(seconds.get())
            }
            Err(_) => Duration::from_secs(DEFAULT_TIMEOUT_SECS),
        };

        Ok(Self {
            api_key,
            base_url,
            model,
            timeout,
        })
    }

    pub fn messages_url(&self) -> Result<Url, ConfigError> {
        self.base_url
            .join("v1/messages")
            .map_err(|source| ConfigError::MessagesUrl { source })
    }
}

pub fn normalize_base_url(value: &str) -> Result<Url, ConfigError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(ConfigError::InvalidBaseUrl {
            value: "<empty>".to_owned(),
            source: url::ParseError::EmptyHost,
        });
    }

    let mut url = Url::parse(value).map_err(|source| ConfigError::InvalidBaseUrl {
        value: value.to_owned(),
        source,
    })?;

    if url.scheme() != "https"
        || url.host_str().is_none()
        || url.username() != ""
        || url.password().is_some()
    {
        return Err(ConfigError::UnsafeBaseUrl);
    }

    let normalized_path = format!("{}/", url.path().trim_end_matches('/'));
    url.set_path(&normalized_path);
    url.set_query(None);
    url.set_fragment(None);

    Ok(url)
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("DEEPSEEK_BASE_URL is invalid")]
    InvalidBaseUrl {
        value: String,
        #[source]
        source: url::ParseError,
    },
    #[error("DEEPSEEK_BASE_URL must be an HTTPS URL without embedded credentials")]
    UnsafeBaseUrl,
    #[error("DEEPSEEK_MODEL must not be empty")]
    EmptyModel,
    #[error("DEEPSEEK_TIMEOUT_SECS must be a positive integer")]
    InvalidTimeout(String),
    #[error("could not construct the Messages endpoint")]
    MessagesUrl {
        #[source]
        source: url::ParseError,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_base_url_without_dropping_path() {
        let url = normalize_base_url("https://api.deepseek.com/anthropic").unwrap();
        assert_eq!(url.as_str(), "https://api.deepseek.com/anthropic/");
        assert_eq!(
            url.join("v1/messages").unwrap().as_str(),
            "https://api.deepseek.com/anthropic/v1/messages"
        );
    }

    #[test]
    fn rejects_base_url_with_credentials_or_http() {
        assert!(matches!(
            normalize_base_url("https://user:secret@example.com/anthropic"),
            Err(ConfigError::UnsafeBaseUrl)
        ));
        assert!(matches!(
            normalize_base_url("http://localhost:8080/anthropic"),
            Err(ConfigError::UnsafeBaseUrl)
        ));
    }

    #[test]
    fn removes_base_url_query_and_fragment() {
        let url =
            normalize_base_url("https://example.com/anthropic?token=secret#fragment").unwrap();
        assert_eq!(url.as_str(), "https://example.com/anthropic/");
    }
}
