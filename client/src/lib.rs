mod filter;

use std::collections::HashMap;

use anyhow::{Ok, Result, bail};

use secrecy::{ExposeSecret, SecretString};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;

pub use erpnext_client_macro::Fieldnames;
pub use filter::Comparator;
pub use filter::FilterValue;
pub struct Client {
    http: reqwest::Client,
    settings: Settings,
}

pub trait Fieldnames {
    fn field_names() -> &'static [&'static str];
}

impl Client {
    pub fn new(s: Settings) -> Self {
        Client {
            http: reqwest::Client::new(),
            settings: s,
        }
    }
    pub fn from_env() -> Result<Self> {
        let s = Settings {
            url: std::env::var("ERPNEXT_URL")?,
            key: std::env::var("ERPNEXT_KEY")?,
            secret: std::env::var("ERPNEXT_SECRET")?.into(),
        };

        Ok(Client {
            http: reqwest::Client::new(),
            settings: s,
        })
    }
    pub fn with_client(http: reqwest::Client, s: Settings) -> Self {
        Self { http, settings: s }
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_doctype_by_name<T: DeserializeOwned + std::fmt::Debug>(
        &self,
        doctype: &str,
        name: &str,
    ) -> Result<Option<T>> {
        let url = format!(
            "{}/api/resource/{}/{}",
            self.settings.url,
            urlencoding::encode(doctype),
            urlencoding::encode(name)
        );
        let response = self
            .http
            .get(url)
            .basic_auth(
                &self.settings.key,
                Some(self.settings.secret.expose_secret()),
            )
            .header("Accept", "application/json")
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;
        if let Some(exc_type) = json.get("exc_type") {
            if exc_type == &serde_json::Value::String("DoesNotExistError".into()) {
                return Ok(None);
            }
        }
        if let Some(exception_value) = json.get("exception") {
            bail!("The response contains an exception: {}", exception_value);
        }

        let data = json
            .get("data")
            .ok_or_else(|| anyhow::anyhow!("Missing 'data' field in response"))?;

        let parsed_data: T = serde_json::from_value(data.clone())?;

        Ok(Some(parsed_data))
    }

    #[tracing::instrument(skip(self, filters))]
    pub async fn list_doctype<T: DeserializeOwned + std::fmt::Debug + Fieldnames>(
        &self,
        doctype: &str,
        filters: HashMap<String, (Comparator, FilterValue)>,
        page_size: Option<usize>,
        page_start: Option<usize>,
    ) -> Result<Vec<T>> {
        const DEFAULT_PAGE_SIZE: usize = 1000;
        const DEFAULT_PAGE_START: usize = 0;
        let fields = serde_json::to_string(T::field_names())?;
        let filters = serde_json::to_string(&filters)?;
        let url = reqwest::Url::parse_with_params(
            format!("{}/api/resource/{}", self.settings.url, doctype).as_str(),
            [
                ("fields", &fields),
                ("filters", &filters),
                (
                    "limit_page_length",
                    &page_size.unwrap_or(DEFAULT_PAGE_SIZE).to_string(),
                ),
                (
                    "limit_start",
                    &page_start.unwrap_or(DEFAULT_PAGE_START).to_string(),
                ),
            ],
        )?;

        let response = self
            .http
            .get(url)
            .basic_auth(
                &self.settings.key,
                Some(self.settings.secret.expose_secret()),
            )
            .header("Accept", "application/json")
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;
        if let Some(exception_value) = json.get("exception") {
            bail!("The response contains an exception: {}", exception_value);
        }

        let data = json
            .get("data")
            .ok_or_else(|| anyhow::anyhow!("Missing 'data' field in response"))?;

        let parsed_data: Vec<T> = serde_json::from_value(data.to_owned())?;

        Ok(parsed_data)
    }

    #[tracing::instrument(skip(self))]
    pub async fn update_doctype<T: Serialize + std::fmt::Debug>(
        &self,
        doctype: &str,
        name: &str,
        data: &T,
    ) -> Result<()> {
        let url = format!("{}/api/resource/{}/{}", self.settings.url, doctype, name);
        let wrapped = json!({"data":data});
        let response = self
            .http
            .put(url)
            .basic_auth(
                &self.settings.key,
                Some(self.settings.secret.expose_secret()),
            )
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&wrapped)
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;
        if let Some(exception_value) = json.get("exception") {
            bail!("The response contains an exception: {}", exception_value);
        }

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn insert_doctype<T: Serialize + std::fmt::Debug>(
        &self,
        doctype: &str,
        data: &T,
    ) -> Result<()> {
        let wrapped = json!({"data":data});
        let url = format!("{}/api/resource/{}", self.settings.url, doctype);
        let response = self
            .http
            .post(url)
            .basic_auth(
                &self.settings.key,
                Some(self.settings.secret.expose_secret()),
            )
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&wrapped)
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;
        if let Some(exception_value) = json.get("exception") {
            bail!("The response contains an exception: {}", exception_value);
        }

        Ok(())
    }
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct Settings {
    pub url: String,
    pub key: String,
    pub secret: SecretString,
}
