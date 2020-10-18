use crate::service::discord::DiscordService;
use std::sync::{Arc, RwLock};

pub struct HomenisState {
    token: String,
    discord: DiscordService,
}

impl HomenisState {
    pub fn new() -> HomenisState {
        let token = std::env::var("HOMENIS_TOKEN").unwrap();
        HomenisState {
            token: token.clone(),
            discord: DiscordService::new(&token, "https://discordapp.com/api"),
        }
    }

    pub fn token(&self) -> String {
        self.token.clone()
    }
}
