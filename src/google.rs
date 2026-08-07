use translators::{GoogleTranslator, Translator};

pub async fn translate(text: &str, source: &str, target: &str) -> Result<String, String> {
    let translator = GoogleTranslator::default();
    translator
        .translate_async(text, source, target)
        .await
        .map_err(|e| e.to_string())
}