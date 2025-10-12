use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

// Default configuration constants
const DEFAULT_TTL_USERS_HOURS: u64 = 24;
const DEFAULT_TTL_CHANNELS_HOURS: u64 = 24;
const DEFAULT_TTL_MEMBERS_HOURS: u64 = 12;
const DEFAULT_COMPRESSION: &str = "snappy";
const DEFAULT_MAX_ATTEMPTS: u32 = 3;
const DEFAULT_INITIAL_DELAY_MS: u64 = 1000;
const DEFAULT_MAX_DELAY_MS: u64 = 60000;
const DEFAULT_EXPONENTIAL_BASE: f64 = 2.0;
const DEFAULT_TIMEOUT_SECONDS: u64 = 30;
const DEFAULT_MAX_IDLE_PER_HOST: i32 = 10;
const DEFAULT_POOL_IDLE_TIMEOUT_SECONDS: u64 = 90;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub slack: SlackConfig,
    pub cache: CacheConfig,
    pub retry: RetryConfig,
    pub connection: ConnectionConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SlackConfig {
    pub bot_token: Option<String>,
    pub user_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CacheConfig {
    pub data_path: String,
    pub ttl_users_hours: u64,
    pub ttl_channels_hours: u64,
    pub ttl_members_hours: u64,
    pub compression: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub exponential_base: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConnectionConfig {
    pub timeout_seconds: u64,
    pub max_idle_per_host: i32,
    pub pool_idle_timeout_seconds: u64,
}

impl Config {
    pub fn load(config_path: Option<&str>, db_path: &str) -> Result<Self> {
        let mut settings = config::Config::builder();

        // Default values
        settings = settings
            .set_default("cache.data_path", db_path)?
            .set_default("cache.ttl_users_hours", DEFAULT_TTL_USERS_HOURS)?
            .set_default("cache.ttl_channels_hours", DEFAULT_TTL_CHANNELS_HOURS)?
            .set_default("cache.ttl_members_hours", DEFAULT_TTL_MEMBERS_HOURS)?
            .set_default("cache.compression", DEFAULT_COMPRESSION)?
            .set_default("retry.max_attempts", DEFAULT_MAX_ATTEMPTS)?
            .set_default("retry.initial_delay_ms", DEFAULT_INITIAL_DELAY_MS)?
            .set_default("retry.max_delay_ms", DEFAULT_MAX_DELAY_MS)?
            .set_default("retry.exponential_base", DEFAULT_EXPONENTIAL_BASE)?
            .set_default("connection.timeout_seconds", DEFAULT_TIMEOUT_SECONDS)?
            .set_default("connection.max_idle_per_host", DEFAULT_MAX_IDLE_PER_HOST)?
            .set_default(
                "connection.pool_idle_timeout_seconds",
                DEFAULT_POOL_IDLE_TIMEOUT_SECONDS,
            )?;

        // Load from config file if provided
        if let Some(path) = config_path
            && Path::new(path).exists()
        {
            settings = settings.add_source(config::File::with_name(path));
        }

        // Override with environment variables
        settings = settings.add_source(
            config::Environment::with_prefix("SLACK")
                .prefix_separator("_")
                .separator("__"),
        );

        // Add Slack tokens from environment (at least one required)
        let bot_token = std::env::var("SLACK_BOT_TOKEN").ok();
        let user_token = std::env::var("SLACK_USER_TOKEN").ok();

        if bot_token.is_none() && user_token.is_none() {
            return Err(anyhow::anyhow!(
                "At least one token required: SLACK_BOT_TOKEN or SLACK_USER_TOKEN"
            ));
        }

        if let Some(token) = bot_token {
            settings = settings.set_override("slack.bot_token", Some(token))?;
        }

        if let Some(token) = user_token {
            settings = settings.set_override("slack.user_token", Some(token))?;
        }

        let config = settings.build()?.try_deserialize()?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    fn setup_test_env() {
        // Clear any existing env vars
        unsafe {
            env::remove_var("SLACK_BOT_TOKEN");
            env::remove_var("SLACK_USER_TOKEN");
        }
    }

    fn cleanup_test_env() {
        unsafe {
            env::remove_var("SLACK_BOT_TOKEN");
            env::remove_var("SLACK_USER_TOKEN");
        }
    }

    #[test]
    #[serial]
    fn test_config_with_bot_token() {
        setup_test_env();
        unsafe {
            env::set_var("SLACK_BOT_TOKEN", "xoxb-test-token");
        }

        let result = Config::load(None, "/tmp/test.db");
        cleanup_test_env();

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.slack.bot_token, Some("xoxb-test-token".to_string()));
        assert_eq!(config.slack.user_token, None);
    }

    #[test]
    #[serial]
    fn test_config_with_user_token() {
        setup_test_env();
        unsafe {
            env::set_var("SLACK_USER_TOKEN", "xoxp-test-token");
        }

        let result = Config::load(None, "/tmp/test.db");
        cleanup_test_env();

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.slack.user_token, Some("xoxp-test-token".to_string()));
        assert_eq!(config.slack.bot_token, None);
    }

    #[test]
    #[serial]
    fn test_config_with_both_tokens() {
        setup_test_env();
        unsafe {
            env::set_var("SLACK_BOT_TOKEN", "xoxb-test-bot");
            env::set_var("SLACK_USER_TOKEN", "xoxp-test-user");
        }

        let result = Config::load(None, "/tmp/test.db");
        cleanup_test_env();

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.slack.bot_token, Some("xoxb-test-bot".to_string()));
        assert_eq!(config.slack.user_token, Some("xoxp-test-user".to_string()));
    }

    #[test]
    #[serial]
    fn test_config_without_tokens_fails() {
        setup_test_env();

        let result = Config::load(None, "/tmp/test.db");
        cleanup_test_env();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string()
                .contains("At least one token required: SLACK_BOT_TOKEN or SLACK_USER_TOKEN")
        );
    }

    #[test]
    #[serial]
    fn test_config_default_cache_values() {
        setup_test_env();
        unsafe {
            env::set_var("SLACK_BOT_TOKEN", "xoxb-test");
        }

        let result = Config::load(None, "/tmp/test.db");
        cleanup_test_env();

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.cache.data_path, "/tmp/test.db");
        assert_eq!(config.cache.ttl_users_hours, DEFAULT_TTL_USERS_HOURS);
        assert_eq!(config.cache.ttl_channels_hours, DEFAULT_TTL_CHANNELS_HOURS);
        assert_eq!(config.cache.ttl_members_hours, DEFAULT_TTL_MEMBERS_HOURS);
        assert_eq!(config.cache.compression, DEFAULT_COMPRESSION);
    }

    #[test]
    #[serial]
    fn test_config_default_retry_values() {
        setup_test_env();
        unsafe {
            env::set_var("SLACK_BOT_TOKEN", "xoxb-test");
        }

        let result = Config::load(None, "/tmp/test.db");
        cleanup_test_env();

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.retry.max_attempts, DEFAULT_MAX_ATTEMPTS);
        assert_eq!(config.retry.initial_delay_ms, DEFAULT_INITIAL_DELAY_MS);
        assert_eq!(config.retry.max_delay_ms, DEFAULT_MAX_DELAY_MS);
        assert_eq!(config.retry.exponential_base, DEFAULT_EXPONENTIAL_BASE);
    }

    #[test]
    #[serial]
    fn test_config_default_connection_values() {
        setup_test_env();
        unsafe {
            env::set_var("SLACK_BOT_TOKEN", "xoxb-test");
        }

        let result = Config::load(None, "/tmp/test.db");
        cleanup_test_env();

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.connection.timeout_seconds, DEFAULT_TIMEOUT_SECONDS);
        assert_eq!(
            config.connection.max_idle_per_host,
            DEFAULT_MAX_IDLE_PER_HOST
        );
        assert_eq!(
            config.connection.pool_idle_timeout_seconds,
            DEFAULT_POOL_IDLE_TIMEOUT_SECONDS
        );
    }

    #[test]
    #[serial]
    fn test_config_with_nonexistent_file() {
        setup_test_env();
        unsafe {
            env::set_var("SLACK_BOT_TOKEN", "xoxb-test");
        }

        // Should not fail when config file doesn't exist
        let result = Config::load(Some("/nonexistent/config.toml"), "/tmp/test.db");
        cleanup_test_env();

        assert!(result.is_ok());
    }
}
