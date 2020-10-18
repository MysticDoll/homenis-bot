use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct DiscordMessage {
    id: String,
}

#[derive(Serialize)]
struct DiscordPostMessage {
    content: String,
}

impl DiscordPostMessage {
    fn new(content: String) -> Self {
        Self { content }
    }
}

#[derive(Clone)]
pub struct DiscordService {
    token: String,
    baseurl: String,
}

impl DiscordService {
    pub fn new(token: &str, baseurl: &str) -> Self {
        Self {
            token: token.to_owned(),
            baseurl: baseurl.to_owned(),
        }
    }

    fn makeUrl(&self, endpoint: &str) -> String {
        format!("{}/{}", self.baseurl, endpoint)
    }

    pub fn post(&self, message: String, channel: String) -> Result<String, String> {
        let url = format!("{}/{}/messages", self.makeUrl("channels"), channel);
        let data = DiscordPostMessage::new(message);

        let resp: DiscordMessage = Client::new()
            .post(&url)
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bot {}", self.token),
            )
            .json(&data)
            .send()
            .map_err(|e| e.to_string())?
            .json()
            .map_err(|e| e.to_string())?;
        Ok(resp.id)
    }
}
