# trslat

Free CLI for Chinese <-> English auto-translation.

## Features

- Chinese <-> English automatic translation with auto-detected target language
- Piped stdin input auto-detected (no flag needed)
- Bing as the default backend (free, no API key); Google available via `-a google`
- Localized help and messages: English or Simplified Chinese, picked from your environment

## Install

Build a release binary and symlink it onto your PATH:

```sh
cargo build --release
ln -sf "$PWD/target/release/trslat" ~/.local/bin/trslat
```

Ensure `~/.local/bin` is on your `PATH`.

## Usage

```sh
trslat "hello world"        # translate an argument
echo "hello world" | trslat # or pipe into stdin
trslat 你好                   # non-ASCII → targets English automatically
```

### Options

```
-s, --source <SOURCE>  Source language code, auto-detect by default
-t, --target <TARGET>  Target language code, e.g. en / zh-CN, auto-detected by default
-v, --verbose          Show request-to-success latency in milliseconds
-a, --api <API>        Translation API: bing (default) or google
```

## Backends

- **Bing** (default): uses cn.bing.com's translator endpoint without an API key.
  Session tokens are cached and refreshed automatically.
- **Google**: wraps the `translators` crate.

## License

TBD