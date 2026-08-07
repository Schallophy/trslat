use regex::Regex;
use reqwest::Client;
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

const TRANSLATE_PAGE: &str = "https://cn.bing.com/translator";
const TRANSLATE_API: &str = "https://cn.bing.com/ttranslatev3?isVertical=1";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/150.0.0.0 Safari/537.36 Edg/151.0.4129.59";

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
    pub async fn new(client: &Client) -> Result<Self, String> {
        if let Some(cached) = BingSession::load_cache() {
            return Ok(cached);
        }
        let fresh = BingSession::fetch(client).await;
        match &fresh {
            Ok(s) => s.save_cache(),
            Err(_) => {}
        }
        fresh
    }

    async fn fetch(client: &Client) -> Result<Self, String> {
        let html = client
            .get(TRANSLATE_PAGE)
            .header("User-Agent", USER_AGENT)
            .send()
            .await
            .map_err(|e| format!("加载翻译页失败：{e}"))?
            .text()
            .await
            .map_err(|e| format!("读取翻译页失败：{e}"))?;

        let re_ig = Regex::new(r#"IG:"([^"]+)""#).unwrap();
        let re_iid = Regex::new(r#"data-iid="([^"]+)""#).unwrap();
        let re_aph = Regex::new(r#"params_AbusePreventionHelper\s*=\s*(\[[^\]]+\])"#).unwrap();

        let ig = re_ig
            .captures(&html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| "未从页面解析到 IG".to_string())?;
        let iid = re_iid
            .captures(&html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| "未从页面解析到 IID".to_string())?;
        let aph_raw = re_aph
            .captures(&html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| "未从页面解析到防滥用令牌".to_string())?;
        let aph: Value =
            serde_json::from_str(&aph_raw).map_err(|e| format!("解析防滥用令牌失败：{e}"))?;
        let key = aph[0].as_i64().unwrap_or_default().to_string();
        let token = aph[1].as_str().unwrap_or_default().to_string();
        let interval_ms = aph[2].as_i64().unwrap_or(3_600_000) as u64;

        if key.is_empty() || token.is_empty() {
            return Err("解析的防滥用令牌为空".into());
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

pub async fn translate(client: &Client, session: &BingSession, text: &str, source: &str, target: &str) -> Result<String, String> {
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
        .map_err(|e| format!("翻译请求失败：{e}"))?
        .json::<Value>()
        .await
        .map_err(|e| format!("解析响应失败：{e}"))?;

    if let Some(err) = body.get("statusCode").and_then(|v| v.as_i64()) {
        return Err(format!("翻译请求被拒绝（状态 {err}）"));
    }

    body[0]["translations"][0]["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| format!("响应格式异常：{body}"))
}

pub async fn translate_smart(client: &Client, text: &str, source: &str, target: &str) -> Result<String, String> {
    let session = BingSession::new(client).await?;
    match translate(client, &session, text, source, target).await {
        Ok(v) => Ok(v),
        Err(_) => {
            let fresh = BingSession::fetch(client).await?;
            translate(client, &fresh, text, source, target).await
        }
    }
}

fn normalize_target(target: &str) -> &str {
    match target {
        "zh" | "zh-CN" => "zh-Hans",
        other => other,
    }
}