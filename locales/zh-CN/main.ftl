about = 免费翻译 CLI：中文 <-> 英文 自动翻译
arg-text = 要翻译的文本
arg-source = 源语言代码，默认自动检测
arg-target = 目标语言代码，如 en / zh-CN，默认自动判断
arg-from-stdin = 从标准输入读取文本
arg-verbose = 显示从请求到翻译成功的耗时（毫秒）
arg-api = 翻译 API：bing（默认）或 google

err-stdin = 错误：从标准输入读取失败
err-no-text = 错误：请提供文本参数，或用 -f 从标准输入读取
err-empty = 错误：输入文本为空
err-no-result = 错误：翻译结果为空，请检查网络连接后重试
err-translate = 错误：翻译失败 – {$error}

verbose = api = {$api}，请求耗时 = {$ms} ms