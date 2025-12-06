#![expect(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct Request {
    pub prompt: String,

    pub model: String,

    pub max_tokens: u32,

    pub temperature: f32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,

    pub stop: &'static [&'static str],
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub text: String,
    pub index: u64,
    pub finish_reason: String,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Default, Deserialize)]
pub struct Response {
    pub id: Option<String>,
    pub object: Option<String>,
    pub created: Option<u64>,
    pub model: Option<String>,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}
