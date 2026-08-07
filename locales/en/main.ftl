about = Free CLI for Chinese <-> English auto translation

arg-text = Text to translate
arg-source = Source language code, auto-detect by default
arg-target = Target language code, e.g. en / zh-CN, auto-detected by default
arg-verbose = Show request-to-success latency in milliseconds
arg-api = Translation API: bing (default) or google

err-stdin = Error: failed to read from standard input
err-no-text = Error: provide the text argument, or read from stdin (e.g. `echo hi | trslat`)
err-empty = Error: input text is empty
err-no-result = Error: translation result is empty, check network and retry
err-network = Error: network request failed – {$error}
err-token = Error: failed to parse Bing anti-abuse token – {$error}
err-rejected = Error: translation request rejected (status {$status})
err-malformed = Error: unexpected response format – {$error}
err-provider = Error: translation provider failed – {$error}

verbose = api = {$api}, latency = {$ms} ms