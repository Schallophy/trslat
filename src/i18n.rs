use std::borrow::Cow;
use std::collections::HashMap;

use clap::Command;
use crate::error::TranslateError;

use fluent_bundle::FluentValue;
use fluent_templates::{LanguageIdentifier, Loader, langid, static_loader};

static_loader! {
    static LOCALES = {
        locales: "./locales",
        fallback_language: "en",
        // Disable Unicode isolation marks (U+2068/U+2069) so interpolated
        // values render cleanly.
        customise: |bundle| bundle.set_use_isolating(false),
    };
}

pub const EN: LanguageIdentifier = langid!("en");
pub const ZH_CN: LanguageIdentifier = langid!("zh-CN");

/// Returns the language of the user-facing messages (only English and
/// Simplified Chinese are supported).
///
/// Precedence detection order:
///  1. `LC_ALL`, `LC_MESSAGES`, `LANG` environment variables (most explicit)
///  2. the operating system's reported locale
pub fn detect() -> LanguageIdentifier {
    let from_env = ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(std::env::var_os)
        .filter_map(|v| v.into_string().ok())
        .find(|v| !v.is_empty() && v != "C" && !v.starts_with("C."));
    pick(from_env.or_else(sys_locale::get_locale).as_deref())
}

/// Map a raw locale string to the supported language. Pure, so it is safe to
/// unit test without touching the environment.
fn pick(raw: Option<&str>) -> LanguageIdentifier {
    match raw.map(|l| l.to_ascii_lowercase()) {
        Some(l) if l.starts_with("zh") => ZH_CN,
        _ => EN,
    }
}

/// Look up a message given its fluent key.
pub fn t(lang: &LanguageIdentifier, key: &str) -> String {
    LOCALES.lookup(lang, key)
}

/// Look up a message with interpolated variables (e.g. `{$api}`, `{$ms}`).
pub fn t_args(lang: &LanguageIdentifier, key: &str, vars: &Args) -> String {
    LOCALES.lookup_with_args(lang, key, &vars.0)
}

/// Localize a `TranslateError` into a user-facing message.
pub fn render_error(lang: &LanguageIdentifier, e: &TranslateError) -> String {
    match e {
        TranslateError::Network(err) => {
            t_args(lang, "err-network", Args::new().set("error", err.clone()))
        }
        TranslateError::TokenParse(err) => {
            t_args(lang, "err-token", Args::new().set("error", err.clone()))
        }
        TranslateError::ApiRejected(status) => {
            t_args(lang, "err-rejected", Args::new().set("status", status.to_string()))
        }
        TranslateError::Malformed(err) => {
            t_args(lang, "err-malformed", Args::new().set("error", err.clone()))
        }
        TranslateError::Provider(err) => {
            t_args(lang, "err-provider", Args::new().set("error", err.clone()))
        }
        TranslateError::Empty => t(lang, "err-empty"),
        TranslateError::NoResult => t(lang, "err-no-result"),
        TranslateError::Stdin => t(lang, "err-stdin"),
        TranslateError::NoInput => t(lang, "err-no-text"),
    }
}

/// Ordered map of variables for interpolation.
#[derive(Default)]
pub struct Args(HashMap<Cow<'static, str>, FluentValue<'static>>);

impl Args {
    pub fn new() -> Self {
        Args(HashMap::new())
    }

    pub fn set(&mut self, name: &'static str, value: impl Into<String>) -> &mut Self {
        self.0.insert(Cow::Borrowed(name), value.into().into());
        self
    }
}

/// Replace the help text of the command and its args with the localized strings.
pub fn localize(mut cmd: Command, lang: &LanguageIdentifier) -> Command {
    cmd = cmd.about(t(lang, "about"));

    let arg_keys = [
        ("text", "arg-text"),
        ("source", "arg-source"),
        ("target", "arg-target"),
        ("verbose", "arg-verbose"),
        ("api", "arg-api"),
    ];
    for (id, key) in arg_keys {
        let help = t(lang, key);
        cmd = cmd.mut_arg(id, |arg| arg.help(help.clone()));
    }
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::TranslateError;

    #[test]
    fn english_messages() {
        assert_eq!(t(&EN, "about"), "Free CLI for Chinese <-> English auto translation");
        assert_eq!(t(&EN, "err-no-text"), "Error: provide the text argument, or read from stdin (e.g. `echo hi | trslat`)");
        assert_eq!(
            t_args(&EN, "verbose", Args::new().set("api", "bing").set("ms", "123")),
            "api = bing, latency = 123 ms"
        );
        assert_eq!(
            render_error(&EN, &TranslateError::ApiRejected(403)),
            "Error: translation request rejected (status 403)"
        );
    }

    #[test]
    fn chinese_messages() {
        assert_eq!(t(&ZH_CN, "about"), "免费翻译 CLI：中文 <-> 英文 自动翻译");
        assert_eq!(t(&ZH_CN, "err-no-text"), "错误：请提供文本参数，或从标准输入读取（如 `echo hi | trslat`）");
        assert_eq!(
            t_args(&ZH_CN, "verbose", Args::new().set("api", "bing").set("ms", "123")),
            "api = bing，请求耗时 = 123 ms"
        );
        assert_eq!(
            render_error(&ZH_CN, &TranslateError::ApiRejected(403)),
            "错误：翻译请求被拒绝（状态 403）"
        );
    }

    #[test]
    fn pick_maps_zh_and_english() {
        assert_eq!(pick(Some("en_US.UTF-8")), EN);
        assert_eq!(pick(Some("zh_CN.UTF-8")), ZH_CN);
        assert_eq!(pick(Some("zh-Hans-CN")), ZH_CN);
        assert_eq!(pick(Some("C")), EN);
        assert_eq!(pick(None), EN);
    }
}