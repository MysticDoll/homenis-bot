use lazy_static::lazy_static;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, RwLock};
use url::Url;
use ws::{CloseCode, Handler, Handshake, Message, Result, Sender};

lazy_static! {
    pub static ref HEARTBEAT_VALUE: Arc<RwLock<Option<u64>>> = Arc::new(RwLock::new(None));
    pub static ref SESSION_ID: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
}

#[derive(Clone)]
pub struct HomenisHandler {
    out: Sender,
    token: String,
    resume: bool,
}

#[derive(Serialize)]
struct IdentifyProperty {
    #[serde(rename = "$os")]
    os: String,
    #[serde(rename = "$browser")]
    browser: String,
    #[serde(rename = "$device")]
    device: String,
}

impl Default for IdentifyProperty {
    fn default() -> Self {
        Self {
            os: "linux".to_owned(),
            browser: "homenis".to_owned(),
            device: "homenis".to_owned(),
        }
    }
}

#[derive(Serialize)]
struct IdentifyData {
    token: String,
    properties: IdentifyProperty,
}

impl Default for IdentifyData {
    fn default() -> Self {
        Self {
            token: "".to_owned(),
            properties: IdentifyProperty::default(),
        }
    }
}

#[derive(Serialize)]
struct IdentifyRequest {
    op: u16,
    d: IdentifyData,
}

impl IdentifyRequest {
    fn token<'a, S: ToString>(&'a mut self, token: S) -> &'a mut Self {
        self.d.token = token.to_string();
        self
    }
}

impl Default for IdentifyRequest {
    fn default() -> Self {
        Self {
            op: 2,
            d: IdentifyData::default(),
        }
    }
}

#[derive(Serialize)]
struct ResumeData {
    token: String,
    session_id: String,
    seq: Option<u64>,
}

#[derive(Serialize)]
struct ResumeRequest {
    op: u16,
    d: ResumeData,
}

#[derive(Serialize)]
struct Heatbeat {
    op: u64,
    d: Option<u64>,
}

impl Heatbeat {
    fn new(d: Option<u64>) -> Self {
        Self { op: 1, d }
    }
}

impl HomenisHandler {
    fn heartbeat(&mut self, interval: u64) {
        let out = self.out.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_millis(interval));
            let v = HEARTBEAT_VALUE.read().unwrap().as_ref().cloned();
            println!(
                "send heartbeat value: {}",
                v.map(|v| v.to_string()).unwrap_or("nil".to_owned())
            );
            if let Err(e) = out.send(serde_json::to_string(&Heatbeat::new(v)).unwrap()) {
                println!("error occured in heartbeat: {}", e);
                break;
            };
        });
    }

    fn resume(&mut self) -> Result<()> {
        if let Some(session_id) = SESSION_ID.read().unwrap().as_ref().cloned() {
            let seq = HEARTBEAT_VALUE.read().unwrap().as_ref().cloned();
            let d = ResumeData {
                token: self.token.clone(),
                session_id,
                seq,
            };
            let req = ResumeRequest { op: 6, d };

            self.out.send(serde_json::to_string(&req).unwrap())
        } else {
            self.identify()
        }
    }

    fn identify(&mut self) -> Result<()> {
        let mut req = IdentifyRequest::default();
        req.token(&self.token);

        self.out.send(serde_json::to_string(&req).unwrap())
    }
}

impl Handler for HomenisHandler {
    fn on_open(&mut self, _: Handshake) -> Result<()> {
        println!("socket opened");
        if self.resume {
            if let Err(e) = self.resume() {
                println!("error occured in resume request: {}", e);
            };
        } else {
            if let Err(e) = self.identify() {
                println!("error occured on identify: {}", e);
            };
        }

        Ok(())
    }

    fn on_message(&mut self, msg: Message) -> Result<()> {
        if let Err(e) = msg
            .into_text()
            .map_err(|e| e.to_string())
            .and_then(|text| serde_json::from_str(&text).map_err(|e| e.to_string()))
            .and_then(|v: Value| {
                if let Some(11) = v.get("op").and_then(|op| op.as_u64()) {
                    if let Some(val) = std::env::var("HONEMIS_DEBUG").ok() {
                        if val == "true" {
                            println!("msg: {}", serde_json::to_string_pretty(&v).unwrap());
                        }
                    }
                } else {
                    println!("msg: {}", serde_json::to_string_pretty(&v).unwrap());
                    if let Some(7) = v.get("op").and_then(|op| op.as_u64()) {
                        return self.out.close(CloseCode::Normal).map_err(|e| e.to_string());
                    }

                    if let Some(9) = v.get("op").and_then(|op| op.as_u64()) {
                        println!("session is invalid. trying to reconnect new session");
                        let mut w = SESSION_ID.write().unwrap();
                        *w = None;
                        return self.out.close(CloseCode::Normal).map_err(|e| e.to_string());
                    }

                    if let Some(10) = v.get("op").and_then(|op| op.as_u64()) {
                        if let Some(d) = v.get("d") {
                            if let Some(interval) =
                                d.get("heartbeat_interval").and_then(|i| i.as_u64())
                            {
                                self.heartbeat(interval);
                            };
                        }
                    }

                    if let Some(n) = v.get("s").and_then(|s| s.as_u64()) {
                        let mut w = HEARTBEAT_VALUE.write().unwrap();
                        *w = Some(n);
                    };

                    if let Some("READY") = v.get("t").and_then(|t| t.as_str()) {
                        if let Some(d) = v.get("d") {
                            if let Some(s) = d.get("session_id").map(|s| s.to_string()) {
                                let mut w = SESSION_ID.write().unwrap();
                                *w = Some(s);
                            }
                        }
                    }
                }
                Ok(())
            })
        {
            println!("error occured on message: {}", e);
        };
        Ok(())
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        let c: u16 = code.into();
        println!("socket closed code: {}, reason: {}", c, reason);
        connect_socket(&self.token, true);
        println!("trying reconnect");
    }

    fn on_error(&mut self, err: ws::Error) {
        println!("error occured: {}", err);
    }
}

#[derive(Deserialize)]
struct GatewayResponse {
    url: Option<String>,
}

pub fn connect_socket<S: ToString>(token: S, resume: bool) {
    let token = token.to_string();
    let resp: GatewayResponse = Client::new()
        .get("https://discordapp.com/api/gateway/bot")
        .header(reqwest::header::AUTHORIZATION, format!("Bot {}", token))
        .send()
        .unwrap()
        .json()
        .unwrap();
    let url = resp.url.unwrap();
    let mut ws = ws::WebSocket::new(move |out| HomenisHandler {
        out,
        token: token.clone(),
        resume,
    })
    .unwrap();

    println!("gateway: {}", url);

    std::thread::spawn(move || {
        ws.connect(Url::parse(&url).unwrap()).unwrap();
        if let Err(e) = ws.run() {
            println!("socket connect failed {}", e);
        };
    });
}
