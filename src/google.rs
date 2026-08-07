use translators::{GoogleTranslator, Translator};

use crate::error::TranslateError;

pub async fn translate(text: &str, source: &str, target: &str) -> Result<String, TranslateError> {
    let translator = GoogleTranslator::default();
    translator
        .translate_async(text, source, target)
        .await
        .map_err(|e| TranslateError::Provider(e.to_string()))
}