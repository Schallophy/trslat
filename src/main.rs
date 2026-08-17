use clap::{CommandFactory, FromArgMatches, Parser, ValueEnum};
use fluent_templates::LanguageIdentifier;
use std::io::{self, IsTerminal, Read};
use std::process::ExitCode;
use std::sync::OnceLock;

use error::TranslateError;

mod bing;
mod error;
mod google;
mod i18n;

#[derive(Clone, Copy, Default, ValueEnum)]
enum Api {
    Google,
    #[default]
    Bing,
}

impl Api {
    fn name(self) -> &'static str {
        match self {
            Api::Google => "google",
            Api::Bing => "bing",
        }
    }
}

#[derive(Parser)]
#[command(name = "trslat", version)]
struct Cli {
    /// Text to translate, or read from stdin when piped
    #[arg(value_name = "TEXT", num_args = 1..)]
    text: Vec<String>,

    /// Target language code, e.g. en / zh-CN, auto-detected by default
    #[arg(short, long)]
    target: Option<String>,

    /// Source language code, auto-detect by default
    #[arg(short, long)]
    source: Option<String>,

    /// Show request-to-success latency in milliseconds
    #[arg(short, long)]
    verbose: bool,

    /// Translation API: bing (default) or google
    #[arg(short = 'a', long, value_enum, default_value_t = Api::Bing)]
    api: Api,
}

/// Whether the text contains CJK characters, driving the auto-target heuristic.
///
/// Covers the CJK Unified Ideographs block plus extension A/B, CJK
/// punctuation, and Japanese kana. A presence check is enough for this
/// heuristic; it does not attempt full language identification.
fn is_cjk(s: &str) -> bool {
    s.chars().any(|c| match c as u32 {
        // CJK Unified Ideographs
        0x4E00..=0x9FFF
        // CJK Extension A
        | 0x3400..=0x4DBF
        // CJK Extension B
        | 0x20000..=0x2A6DF
        // CJK punctuation
        | 0x3000..=0x303F
        // Hiragana / Katakana
        | 0x3040..=0x30FF => true,
        _ => false,
    })
}

async fn run(cli: &Cli) -> Result<String, TranslateError> {
    let text = match (cli.text.is_empty(), !io::stdin().is_terminal()) {
        (false, _) => cli.text.join(" "),
        (true, true) => {
            let mut buf = String::new();
            io::stdin()
                .read_to_string(&mut buf)
                .map_err(|_| TranslateError::Stdin)?;
            buf
        }
        (true, false) => return Err(TranslateError::NoInput),
    };

    let text = text.trim().to_string();
    if text.is_empty() {
        return Err(TranslateError::Empty);
    }

    let target = match &cli.target {
        Some(t) => t.clone(),
        None => {
            if is_cjk(&text) {
                "en".to_string()
            } else {
                "zh-CN".to_string()
            }
        }
    };

    let source = cli.source.clone().unwrap_or_default();

    match cli.api {
        Api::Google => google::translate(&text, &source, &target).await,
        Api::Bing => bing::translate_smart(shared_client(), &text, &source, &target).await,
    }
}

/// Lazily shared HTTP client, reused across the initial request and any retry.
fn shared_client() -> &'static reqwest::Client {
    static HTTP: OnceLock<reqwest::Client> = OnceLock::new();
    HTTP.get_or_init(reqwest::Client::new)
}

#[tokio::main]
async fn main() -> ExitCode {
    let locale = i18n::detect();
    let cli = Cli::from_arg_matches(&i18n::localize(Cli::command(), &locale).get_matches())
        .unwrap_or_else(|e| e.exit());

    let start = std::time::Instant::now();
    let outcome = run(&cli)
        .await
        .and_then(|output| match output.trim() {
            "" => Err(TranslateError::NoResult),
            trimmed => Ok(trimmed.to_string()),
        });
    let elapsed_ms = start.elapsed().as_millis();

    if cli.verbose {
        print_verbose(&locale, cli.api.name(), elapsed_ms);
    }

    match outcome {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}", i18n::render_error(&locale, &e));
            ExitCode::FAILURE
        }
    }
}

fn print_verbose(locale: &LanguageIdentifier, api: &str, elapsed_ms: u128) {
    let msg = i18n::t_args(
        locale,
        "verbose",
        i18n::Args::new()
            .set("ms", format!("{elapsed_ms}"))
            .set("api", api),
    );
    eprintln!("[trslat] {msg}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cjk_detection() {
        assert!(is_cjk("你好"));
        assert!(is_cjk("早"));
        assert!(is_cjk("𠮷")); // CJK Extension B
        assert!(is_cjk("「引号」")); // CJK punctuation
        assert!(is_cjk("こんにちは")); // Hiragana
        assert!(is_cjk("hello 世界"));
        assert!(!is_cjk("hello world"));
        assert!(!is_cjk("12345"));
    }
}
