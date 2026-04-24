//! "Sign in with Claude" — piggy-back on the student's Claude Pro/Max
//! subscription instead of asking them for a second BYOK key.
//!
//! Implementation: detect credentials previously stored by Claude Code
//! (`claude login`), either as a JSON file at `~/.claude/.credentials.json`
//! or in the OS keychain under a known service name. If the access token
//! is still valid, Datlino uses it as a Bearer credential on the
//! Anthropic Messages API — usage counts against the user's own
//! subscription quota, not Datlino's bill.
//!
//! Refresh of the OAuth access token requires Anthropic's OAuth client
//! ID — not something Datlino ships. When the token expires we surface a
//! helpful message asking the student to re-run `claude login`; we don't
//! silently guess at the refresh endpoint.
//!
//! Fallback order in `rephrase.rs`:
//!   1. Claude Code OAuth token (if present and not expired)
//!   2. BYOK Anthropic API key in Datlino's keyring
//!   3. error

use anyhow::Result;
use serde::Deserialize;
use std::path::PathBuf;

const KEYCHAIN_SERVICE_CANDIDATES: &[&str] =
    &["Claude Code-credentials", "com.anthropic.claude-code", "Claude Code"];
const KEYCHAIN_USER: &str = "default";

#[derive(Debug, Clone, Deserialize)]
struct CredsFile {
    // Claude Code nests credentials under "claudeAiOauth"; some older
    // builds store them at the top level. We tolerate both shapes.
    #[serde(rename = "claudeAiOauth")]
    oauth_nested: Option<OAuthData>,
    #[serde(flatten)]
    oauth_flat: Option<OAuthData>,
}

#[derive(Debug, Clone, Deserialize)]
struct OAuthData {
    #[serde(alias = "accessToken", alias = "access_token")]
    access_token: Option<String>,
    #[serde(alias = "refreshToken", alias = "refresh_token")]
    #[allow(dead_code)]
    refresh_token: Option<String>,
    #[serde(alias = "expiresAt", alias = "expires_at")]
    expires_at_ms: Option<u64>,
    #[serde(alias = "subscriptionType", alias = "subscription_type")]
    subscription_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ClaudeSubscription {
    pub access_token: String,
    pub subscription_type: Option<String>,
    pub expires_at_ms: Option<u64>,
    pub source: Source,
}

#[derive(Debug, Clone, Copy)]
pub enum Source {
    File,
    Keychain,
}

/// Return the student's Claude OAuth credentials, or None if we can't
/// find any. Never panics on a missing file or a malformed credential
/// blob; those map to "not detected".
pub fn detect() -> Option<ClaudeSubscription> {
    if let Some(sub) = detect_file() {
        return Some(sub);
    }
    detect_keychain()
}

fn detect_file() -> Option<ClaudeSubscription> {
    let home = home_dir()?;
    let creds_path = home.join(".claude").join(".credentials.json");
    let raw = std::fs::read_to_string(&creds_path).ok()?;
    let parsed: CredsFile = serde_json::from_str(&raw).ok()?;
    let oauth = parsed.oauth_nested.or(parsed.oauth_flat)?;
    let token = oauth.access_token?;
    Some(ClaudeSubscription {
        access_token: token,
        subscription_type: oauth.subscription_type,
        expires_at_ms: oauth.expires_at_ms,
        source: Source::File,
    })
}

fn detect_keychain() -> Option<ClaudeSubscription> {
    for service in KEYCHAIN_SERVICE_CANDIDATES {
        let Ok(entry) = keyring::Entry::new(service, KEYCHAIN_USER) else {
            continue;
        };
        let Ok(raw) = entry.get_password() else {
            continue;
        };
        // Keychain entry may be the raw JSON blob Claude Code wrote, or
        // just the access token alone. Try JSON first.
        if let Ok(parsed) = serde_json::from_str::<CredsFile>(&raw) {
            if let Some(oauth) = parsed.oauth_nested.or(parsed.oauth_flat) {
                if let Some(token) = oauth.access_token {
                    return Some(ClaudeSubscription {
                        access_token: token,
                        subscription_type: oauth.subscription_type,
                        expires_at_ms: oauth.expires_at_ms,
                        source: Source::Keychain,
                    });
                }
            }
        }
        // Bare token — we have no expiry info but the request will 401
        // when it runs out, which we treat as "need to re-login".
        if raw.starts_with("sk-ant-oat") {
            return Some(ClaudeSubscription {
                access_token: raw,
                subscription_type: None,
                expires_at_ms: None,
                source: Source::Keychain,
            });
        }
    }
    None
}

pub fn is_expired(sub: &ClaudeSubscription) -> bool {
    let Some(exp) = sub.expires_at_ms else {
        return false; // no expiry → assume live
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    exp < now
}

fn home_dir() -> Option<PathBuf> {
    #[cfg(any(unix, target_os = "macos"))]
    {
        std::env::var_os("HOME").map(PathBuf::from)
    }
    #[cfg(windows)]
    {
        std::env::var_os("USERPROFILE").map(PathBuf::from)
    }
}

#[derive(Debug, serde::Serialize)]
pub struct SubscriptionStatus {
    pub detected: bool,
    pub expired: bool,
    pub subscription_type: Option<String>,
    pub source: Option<String>,
}

pub fn status() -> Result<SubscriptionStatus> {
    let sub = detect();
    Ok(match sub {
        None => SubscriptionStatus {
            detected: false,
            expired: false,
            subscription_type: None,
            source: None,
        },
        Some(s) => SubscriptionStatus {
            detected: true,
            expired: is_expired(&s),
            subscription_type: s.subscription_type.clone(),
            source: Some(match s.source {
                Source::File => "file".into(),
                Source::Keychain => "keychain".into(),
            }),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_returns_cleanly_when_no_claude_code_installed() {
        // In CI / headless build box, neither the file nor keychain
        // should contain Datlino-relevant Claude creds. Status must
        // report `detected: false` without panicking.
        let s = status().unwrap();
        // We can't assert detected=false because whoever runs this may
        // have Claude Code installed. Just assert the call works.
        let _ = (s.detected, s.expired, s.subscription_type, s.source);
    }

    #[test]
    fn is_expired_respects_absent_expiry_as_not_expired() {
        let sub = ClaudeSubscription {
            access_token: "x".into(),
            subscription_type: None,
            expires_at_ms: None,
            source: Source::File,
        };
        assert!(!is_expired(&sub));
    }

    #[test]
    fn is_expired_flags_past_timestamps() {
        let sub = ClaudeSubscription {
            access_token: "x".into(),
            subscription_type: None,
            expires_at_ms: Some(1), // 1ms past epoch, obviously expired
            source: Source::File,
        };
        assert!(is_expired(&sub));
    }
}
