use serde::{Deserialize, Serialize};

/// Words for the user in both of the app's languages; Settings shows the one
/// it reads in, so a reason stays right after the language changes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Text {
    pub zh: String,
    pub en: String,
}

impl Text {
    pub fn new(zh: impl Into<String>, en: impl Into<String>) -> Self {
        Self {
            zh: zh.into(),
            en: en.into(),
        }
    }
}

impl std::fmt::Display for Text {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.en)
    }
}

pub(crate) fn text(zh: impl Into<String>, en: impl Into<String>) -> Text {
    Text::new(zh, en)
}
