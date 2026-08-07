use clap::{Parser, ValueEnum};
use std::io::{self, Read};
use std::process::ExitCode;
use translators::{GoogleTranslator, Translator};

mod bing;

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
#[command(name = "trslat", version, about = "免费翻译 CLI：中文 <-> 英文 自动翻译")]
struct Cli {
    /// 要翻译的文本
    #[arg(value_name = "TEXT")]
    text: Option<String>,

    /// 目标语言代码，如 en / zh-CN，默认自动判断
    #[arg(short, long)]
    target: Option<String>,

    /// 源语言代码，默认自动检测
    #[arg(short, long)]
    source: Option<String>,

    /// 从标准输入读取文本
    #[arg(short, long)]
    from_stdin: bool,

    /// 显示从开始请求到翻译成功的耗时（毫秒）
    #[arg(short, long)]
    verbose: bool,

    /// 翻译 API：bing（默认）或 google
    #[arg(short = 'a', long, value_enum, default_value_t = Api::Bing)]
    api: Api,
}

fn is_chinese(s: &str) -> bool {
    s.chars()
        .any(|c| (0x4E00..=0x9FFF).contains(&(c as u32)))
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    let text = match (&cli.text, cli.from_stdin) {
        (Some(t), _) => t.clone(),
        (None, true) => {
            let mut buf = String::new();
            if io::stdin().read_to_string(&mut buf).is_err() {
                eprintln!("错误：从标准输入读取失败");
                return ExitCode::FAILURE;
            }
            buf
        }
        (None, false) => {
            eprintln!("错误：请提供文本参数，或用 -f 从标准输入读取");
            return ExitCode::FAILURE;
        }
    };

    let text = text.trim().to_string();
    if text.is_empty() {
        eprintln!("错误：输入文本为空");
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
        Api::Google => {
            let translator = GoogleTranslator::default();
            translator
                .translate_async(&text, &source, &target)
                .await
                .map_err(|e| e.to_string())
        }
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
                eprintln!("错误：翻译结果为空，请检查网络连接后重试");
                return ExitCode::FAILURE;
            }
            if cli.verbose {
                eprintln!("[trslat] api = {}，请求到翻译成功耗时：{elapsed_ms} ms", cli.api.name());
            }
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            if cli.verbose {
                eprintln!("[trslat] api = {}，请求耗时：{elapsed_ms} ms", cli.api.name());
            }
            eprintln!("错误：翻译失败 – {e}");
            ExitCode::FAILURE
        }
    }
}