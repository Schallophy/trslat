use clap::{CommandFactory, FromArgMatches, Parser, ValueEnum};
use std::io::{self, IsTerminal, Read};
use std::process::ExitCode;

mod bing;
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
    /// placeholder
    #[arg(value_name = "TEXT")]
    text: Option<String>,

    /// placeholder
    #[arg(short, long)]
    target: Option<String>,

    /// placeholder
    #[arg(short, long)]
    source: Option<String>,

    /// placeholder
    #[arg(short, long)]
    verbose: bool,

    /// placeholder
    #[arg(short = 'a', long, value_enum, default_value_t = Api::Bing)]
    api: Api,
}

fn is_chinese(s: &str) -> bool {
    s.chars().any(|c| (0x4E00..=0x9FFF).contains(&(c as u32)))
}

#[tokio::main]
async fn main() -> ExitCode {
    let locale = i18n::detect();
    let cli = Cli::from_arg_matches(&i18n::localize(Cli::command(), &locale).get_matches())
        .unwrap_or_else(|e| e.exit());

    let text = match (&cli.text, !io::stdin().is_terminal()) {
        (Some(t), _) => t.clone(),
        (None, true) => {
            let mut buf = String::new();
            if io::stdin().read_to_string(&mut buf).is_err() {
                eprintln!("{}", i18n::t(&locale, "err-stdin"));
                return ExitCode::FAILURE;
            }
            buf
        }
        (None, false) => {
            eprintln!("{}", i18n::t(&locale, "err-no-text"));
            return ExitCode::FAILURE;
        }
    };

    let text = text.trim().to_string();
    if text.is_empty() {
        eprintln!("{}", i18n::t(&locale, "err-empty"));
        return ExitCode::FAILURE;
    }

    let target = match &cli.target {
        Some(t) => t.clone(),
        None => {
            if is_chinese(&text) {
                "en".to_string()
            } else {
                "zh-CN".to_string()
            }
        }
    };

    let source = cli.source.clone().unwrap_or_default();

    let start = std::time::Instant::now();
    let result: Result<String, String> = match cli.api {
        Api::Google => google::translate(&text, &source, &target).await,
        Api::Bing => {
            let client = reqwest::Client::new();
            bing::translate_smart(&client, &text, &source, &target).await
        }
    };
    let elapsed_ms = start.elapsed().as_millis();

    match result {
        Ok(result) => {
            let output = result.trim();
            if output.is_empty() {
                eprintln!("{}", i18n::t(&locale, "err-no-result"));
                return ExitCode::FAILURE;
            }
            if cli.verbose {
                let msg = i18n::t_args(
                    &locale,
                    "verbose",
                    i18n::Args::new()
                        .set("ms", format!("{elapsed_ms}"))
                        .set("api", cli.api.name()),
                );
                eprintln!("[trslat] {msg}");
            }
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            if cli.verbose {
                let msg = i18n::t_args(
                    &locale,
                    "verbose",
                    i18n::Args::new()
                        .set("ms", format!("{elapsed_ms}"))
                        .set("api", cli.api.name()),
                );
                eprintln!("[trslat] {msg}");
            }
            eprintln!(
                "{}",
                i18n::t_args(&locale, "err-translate", &i18n::Args::new().set("error", e))
            );
            ExitCode::FAILURE
        }
    }
}
