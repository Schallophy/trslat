use regex::Regex;
use reqwest::Client;
use serde_json::{json, Value};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::TranslateError;

const TRANSLATE_PAGE: &str = "https://cn.bing.com/translator";
const TRANSLATE_API: &str = "https://cn.bing.com/ttranslatev3?isVertical=1";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/150.0.0.0 Safari/537.36 Edg/151.0.4129.59";

fn re_ig() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"IG:"([^"]+)""#).unwrap())
}

fn re_iid() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"data-iid="([^"]+)""#).unwrap())
}

fn re_aph() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"params_AbusePreventionHelper\s*=\s*(\[[^\]]+\])"#).unwrap())
}

pub struct BingSession {
    ig: String,
    iid: String,
    key: String,
    token: String,
    expires_at: u64,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl BingSession {
    pub async fn new(client: &Client) -> Result<Self, TranslateError> {
        if let Some(cached) = BingSession::load_cache() {
            return Ok(cached);
        }
        let fresh = BingSession::fetch(client).await;
        if let Ok(s) = &fresh {
            s.save_cache();
        }
        fresh
    }

    async fn fetch(client: &Client) -> Result<Self, TranslateError> {
        let html = client
            .get(TRANSLATE_PAGE)
            .header("User-Agent", USER_AGENT)
            .send()
            .await
            .map_err(|e| TranslateError::Network(format!("load translator page: {e}")))?
            .text()
            .await
            .map_err(|e| TranslateError::Network(format!("read translator page: {e}")))?;
        Self::parse_session(&html)
    }

    fn parse_session(html: &str) -> Result<Self, TranslateError> {
        let ig = re_ig()
            .captures(html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| TranslateError::TokenParse("IG not found".into()))?;
        let iid = re_iid()
            .captures(html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| TranslateError::TokenParse("IID not found".into()))?;
        let aph_raw = re_aph()
            .captures(html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| TranslateError::TokenParse("anti-abuse token not found".into()))?;
        let aph: Value = serde_json::from_str(&aph_raw)
            .map_err(|e| TranslateError::TokenParse(format!("parse token JSON: {e}")))?;
        let key = aph[0].as_i64().unwrap_or_default().to_string();
        let token = aph[1].as_str().unwrap_or_default().to_string();
        let interval_ms = aph[2].as_i64().unwrap_or(3_600_000) as u64;

        if key.is_empty() || token.is_empty() {
            return Err(TranslateError::TokenParse("parsed token is empty".into()));
        }

        Ok(BingSession {
            ig,
            iid,
            key,
            token,
            expires_at: now_ms() + interval_ms,
        })
    }

    fn cache_path() -> std::path::PathBuf {
        std::env::temp_dir().join("trslat_bing_session.json")
    }

    fn save_cache(&self) {
        let data = json!({
            "ig": self.ig,
            "iid": self.iid,
            "key": self.key,
            "token": self.token,
            "expires_at": self.expires_at,
        });
        if let Ok(json) = serde_json::to_string(&data) {
            let _ = std::fs::write(BingSession::cache_path(), json);
        }
    }

    fn load_cache() -> Option<BingSession> {
        let raw = std::fs::read_to_string(BingSession::cache_path()).ok()?;
        let v: Value = serde_json::from_str(&raw).ok()?;
        let expires_at = v.get("expires_at").and_then(|x| x.as_u64())?;
        if now_ms() + 60_000 >= expires_at {
            return None;
        }
        Some(BingSession {
            ig: v.get("ig")?.as_str()?.to_string(),
            iid: v.get("iid")?.as_str()?.to_string(),
            key: v.get("key")?.as_str()?.to_string(),
            token: v.get("token")?.as_str()?.to_string(),
            expires_at,
        })
    }
}

pub async fn translate(
    client: &Client,
    session: &BingSession,
    text: &str,
    source: &str,
    target: &str,
) -> Result<String, TranslateError> {
    let x_target = normalize_target(target);
    let x_source = if source.is_empty() { "auto-detect" } else { source };

    let body: Value = client
        .post(format!("{}&IG={}&IID={}", TRANSLATE_API, session.ig, session.iid))
        .header("User-Agent", USER_AGENT)
        .header("Referer", TRANSLATE_PAGE)
        .form(&[
            ("fromLang", x_source),
            ("text", text),
            ("to", x_target),
            ("token", session.token.as_str()),
            ("key", session.key.as_str()),
        ])
        .send()
        .await
        .map_err(|e| TranslateError::Network(format!("translation request failed: {e}")))?
        .json::<Value>()
        .await
        .map_err(|e| TranslateError::Network(format!("parse response: {e}")))?;

    parse_translation(&body)
}

fn parse_translation(body: &Value) -> Result<String, TranslateError> {
    if let Some(status) = body.get("statusCode").and_then(|v| v.as_i64()) {
        return Err(TranslateError::ApiRejected(status));
    }
    body[0]["translations"][0]["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| TranslateError::Malformed(body.to_string()))
}

pub async fn translate_smart(
    client: &Client,
    text: &str,
    source: &str,
    target: &str,
) -> Result<String, TranslateError> {
    let session = BingSession::new(client).await?;
    match translate(client, &session, text, source, target).await {
        Ok(v) => Ok(v),
        Err(e) if retryable(&e) => {
            let fresh = BingSession::fetch(client).await?;
            translate(client, &fresh, text, source, target).await
        }
        Err(e) => Err(e),
    }
}

/// A translation failure worth recovering from by refreshing the session.
///
/// Network errors are not retried: a fresh session cannot fix a dead
/// connection, and retrying would only waste a request.
fn retryable(e: &TranslateError) -> bool {
    matches!(
        e,
        TranslateError::ApiRejected(_) | TranslateError::Malformed(_) | TranslateError::TokenParse(_)
    )
}

fn normalize_target(target: &str) -> &str {
    match target {
        "zh" | "zh-CN" => "zh-Hans",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_HTML: &str = r#"IG:"A1B2C3" data-iid="translator.5024" params_AbusePreventionHelper = [12345,"tok123",3600000];"#;

    #[test]
    fn parses_session_from_html() {
        let s = BingSession::parse_session(SAMPLE_HTML).unwrap();
        assert_eq!(s.ig, "A1B2C3");
        assert_eq!(s.iid, "translator.5024");
        assert_eq!(s.key, "12345");
        assert_eq!(s.token, "tok123");
    }

    #[test]
    fn session_parse_errors_missing_parts() {
        assert!(BingSession::parse_session("no tokens here").is_err());
    }

    #[test]
    fn parses_success_translation() {
        let v = json!([{"translations":[{"text":"你好"}]}]);
        assert_eq!(parse_translation(&v).unwrap(), "你好");
    }

    #[test]
    fn translation_rejected_status() {
        let v = json!({"statusCode": 403});
        assert!(matches!(parse_translation(&v), Err(TranslateError::ApiRejected(403))));
    }

    #[test]
    fn translation_malformed() {
        let v = json!({"unexpected": true});
        assert!(matches!(parse_translation(&v), Err(TranslateError::Malformed(_))));
    }

    #[test]
    fn normalizes_target() {
        assert_eq!(normalize_target("zh"), "zh-Hans");
        assert_eq!(normalize_target("zh-CN"), "zh-Hans");
        assert_eq!(normalize_target("en"), "en");
    }
}