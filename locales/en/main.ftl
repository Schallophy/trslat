about = Free CLI for Chinese <-> English auto translation

arg-text = Text to translate
arg-source = Source language code, auto-detect by default
arg-target = Target language code, e.g. en / zh-CN, auto-detected by default
arg-stdin = Read text from standard input
arg-verbose = Show request-to-success latency in milliseconds
arg-api = Translation API: bing (default) or google

err-stdin = Error: failed to read from standard input
err-no-text = Error: provide the text argument, or use -f to read from stdin
err-empty = Error: input text is empty
err-no-result = Error: translation result is empty, check network and retry
err-translate = Error: translation failed – {$error}

verbose = api = {$api}, latency = {$ms} ms