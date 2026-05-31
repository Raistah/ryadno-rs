use rkyv::Archive;
use serde::Serialize;

#[derive(Archive, rkyv::Serialize, rkyv::Deserialize, Debug, PartialEq, Eq)]
pub struct FormContent {
    pub align: FormContentAlign,
    pub parts: Vec<FormContentPart>,
}

impl Default for FormContent {
    fn default() -> Self {
        Self {
            align: FormContentAlign::Start,
            parts: Vec::new(),
        }
    }
}

impl FormContent {
    pub fn new(align: FormContentAlign, parts: Vec<FormContentPart>) -> Self {
        Self { align, parts }
    }

    pub fn to_html(&self) -> Option<String> {
        if self.parts.len() == 0 {
            None
        } else {
            Some(
                self.parts
                    .iter()
                    .map(|item| item.to_html())
                    .collect::<Vec<String>>()
                    .join(""),
            )
        }
    }
}

impl From<Vec<FormContentPart>> for FormContent {
    fn from(value: Vec<FormContentPart>) -> Self {
        Self {
            align: FormContentAlign::Start,
            parts: value,
        }
    }
}

#[derive(Archive, rkyv::Serialize, rkyv::Deserialize, Debug, PartialEq, Eq)]
pub enum FormContentPart {
    HTML(String),
}

impl FormContentPart {
    pub fn to_html(&self) -> String {
        match self {
            Self::HTML(v) => v.clone(),
        }
    }
}

impl From<String> for FormContentPart {
    fn from(value: String) -> Self {
        FormContentPart::HTML(value)
    }
}

impl From<&str> for FormContentPart {
    fn from(value: &str) -> Self {
        FormContentPart::HTML(value.to_string())
    }
}

#[derive(Archive, rkyv::Serialize, rkyv::Deserialize, Debug, PartialEq, Eq, Serialize)]
pub enum FormContentAlign {
    Start,
    Center,
    End,
}
