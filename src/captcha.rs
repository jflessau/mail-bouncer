use crate::error::{Error, Result};
use captcha::{
    filters::{Grid, Noise, Wave},
    Captcha as CaptchaImg,
};
use chrono::{DateTime, Utc};
use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, Mutex},
};

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct Captcha {
    text: String,
    created_at: DateTime<Utc>,
}

impl Captcha {
    fn new(text: String) -> Self {
        Captcha {
            text: text.to_ascii_lowercase(),
            created_at: Utc::now(),
        }
    }

    fn matches(&self, text: &str) -> bool {
        !self.expired() && self.text == text.to_lowercase()
    }

    fn expired(&self) -> bool {
        (Utc::now() - self.created_at).num_minutes() > 30
    }
}

impl fmt::Display for Captcha {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "text: {}, created_at: {}, expired: {}",
            self.text,
            self.created_at,
            self.expired()
        )
    }
}

type CaptchasList = Arc<Mutex<HashMap<String, Captcha>>>;

#[derive(Clone, Debug)]
pub struct Captchas(CaptchasList);

impl Captchas {
    pub fn new() -> Self {
        Captchas(Arc::new(Mutex::new(HashMap::new())))
    }

    pub fn insert(&self) -> Result<Vec<u8>> {
        let mut captcha = CaptchaImg::new();
        captcha.add_chars(6);
        let text = captcha
            .chars()
            .into_iter()
            .collect::<String>()
            .to_lowercase();

        let image = captcha
            .apply_filter(Noise::new(0.2))
            .apply_filter(Wave::new(2.0, 8.0).horizontal())
            .apply_filter(Grid::new(9, 9))
            .view(200, 64)
            .as_png()
            .ok_or_else(|| Error::InternalServer("fails to build captcha image".to_string()))?;

        if let Ok(mut captchas) = self.0.lock() {
            captchas.retain(|_, v| !v.expired());

            let captcha = Captcha::new(text.clone());
            captchas.insert(text, captcha);

            return Ok(image);
        }

        Err(Error::InternalServer("fails to lock mutex".to_string()))
    }

    pub fn check(&mut self, text: String) -> Result<()> {
        if let Ok(mut captchas) = self.0.lock() {
            if let Some(captcha) = captchas.get(&text) {
                if captcha.matches(&text) {
                    captchas.retain(|k, _| k != &text);
                    return Ok(());
                }
            }
            return Err(Error::Unauthorized);
        }

        Err(Error::InternalServer("fails to lock mutex".to_string()))
    }
}
