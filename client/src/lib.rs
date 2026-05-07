mod filter;

use secrecy::{ExposeSecret, SecretString};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;

pub use reqwest;

pub use erpnext_client_macro::Fieldnames;
pub use filter::Comparator;
pub use filter::FilterValue;
pub use filter::Filters;
pub use filter::IntoFilterValue;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("failed to read environment variable `{name}`: {source}")]
    EnvVar {
        name: &'static str,
        #[source]
        source: std::env::VarError,
    },
    #[error("invalid settings: {0}")]
    InvalidSettings(String),
    #[error("failed to parse URL `{url}`: {source}")]
    UrlParse {
        url: String,
        #[source]
        source: url::ParseError,
    },
    #[error("failed to build HTTP request: {source}")]
    RequestBuild {
        #[source]
        source: reqwest::Error,
    },
    #[error("HTTP request failed: {source}")]
    Http {
        #[source]
        source: reqwest::Error,
    },
    #[error("failed to encode JSON for {context}: {source}")]
    JsonEncode {
        context: &'static str,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to decode JSON response for doctype `{doctype}`: {source}")]
    JsonDecode {
        doctype: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to decode `data` payload for doctype `{doctype}`: {source}")]
    DataDecode {
        doctype: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("ERPNext returned HTTP {status} for doctype `{doctype}` (parent={parent:?}): {body}")]
    HttpStatus {
        status: reqwest::StatusCode,
        doctype: String,
        parent: Option<String>,
        body: String,
    },
    #[error("ERPNext response missing `data` field for doctype `{doctype}`")]
    MissingData { doctype: String },
    #[error("permission denied for doctype `{doctype}` (parent={parent:?}): {frappe_exception}")]
    PermissionDenied {
        doctype: String,
        parent: Option<String>,
        frappe_exception: String,
    },
    #[error("ERPNext exception for doctype `{doctype}` (parent={parent:?}): {frappe_exception}")]
    ErpException {
        doctype: String,
        parent: Option<String>,
        frappe_exception: String,
    },
}

#[derive(Debug, Clone)]
pub struct ListRequest {
    doctype: String,
    filters: Filters,
    page_size: Option<usize>,
    page_start: Option<usize>,
    parent: Option<String>,
}

impl ListRequest {
    #[must_use]
    pub fn new(doctype: impl Into<String>) -> Self {
        Self {
            doctype: doctype.into(),
            filters: Filters::default(),
            page_size: None,
            page_start: None,
            parent: None,
        }
    }

    #[must_use]
    pub fn builder(doctype: impl Into<String>) -> ListRequestBuilder {
        ListRequestBuilder {
            request: Self::new(doctype),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ListRequestBuilder {
    request: ListRequest,
}

impl ListRequestBuilder {
    #[must_use]
    pub fn filters(mut self, filters: impl Into<Filters>) -> Self {
        self.request.filters = filters.into();
        self
    }

    #[must_use]
    pub fn page_size(mut self, page_size: usize) -> Self {
        self.request.page_size = Some(page_size);
        self
    }

    #[must_use]
    pub fn page_size_opt(mut self, page_size: Option<usize>) -> Self {
        self.request.page_size = page_size;
        self
    }

    #[must_use]
    pub fn page_start(mut self, page_start: usize) -> Self {
        self.request.page_start = Some(page_start);
        self
    }

    #[must_use]
    pub fn page_start_opt(mut self, page_start: Option<usize>) -> Self {
        self.request.page_start = page_start;
        self
    }

    #[must_use]
    pub fn parent(mut self, parent: impl Into<String>) -> Self {
        self.request.parent = Some(parent.into());
        self
    }

    #[must_use]
    pub fn parent_opt(mut self, parent: Option<String>) -> Self {
        self.request.parent = parent;
        self
    }

    #[must_use]
    pub fn build(self) -> ListRequest {
        self.request
    }
}

pub struct Client {
    http: reqwest::Client,
    settings: Settings,
}

// Only used if you do not care about the response
#[derive(Debug, Deserialize)]
struct Blank {}

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
        let url = std::env::var("ERPNEXT_URL").map_err(|source| Error::EnvVar {
            name: "ERPNEXT_URL",
            source,
        })?;

        if url.ends_with('/') {
            return Err(Error::InvalidSettings(
                "url cannot end with a /".to_string(),
            ));
        }

        let s = Settings {
            url,
            key: std::env::var("ERPNEXT_KEY").map_err(|source| Error::EnvVar {
                name: "ERPNEXT_KEY",
                source,
            })?,
            secret: std::env::var("ERPNEXT_SECRET")
                .map_err(|source| Error::EnvVar {
                    name: "ERPNEXT_SECRET",
                    source,
                })?
                .into(),
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
    pub async fn find_doctype_by_name<T: DeserializeOwned + std::fmt::Debug>(
        &self,
        doctype: &str,
        name: &str,
    ) -> Result<Option<T>> {
        let url = self.resource_url_with_name(doctype, name)?;

        let request = self
            .http
            .get(url)
            .basic_auth(
                &self.settings.key,
                Some(self.settings.secret.expose_secret()),
            )
            .header("Accept", "application/json")
            .build()
            .map_err(|source| Error::RequestBuild { source })?;

        let response = self.log_http_request(request).await?;
        let (status, json, body) = self.decode_json_response(response, doctype, None).await?;

        if Self::is_missing_doctype_response(status, &json) {
            return Ok(None);
        }

        Self::ensure_erp_success(doctype, None, status, &body, &json)?;

        let data = Self::extract_data(doctype, &json)?;

        let parsed_data: T =
            serde_json::from_value(data.clone()).map_err(|source| Error::DataDecode {
                doctype: doctype.to_string(),
                source,
            })?;

        Ok(Some(parsed_data))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_doctype_by_name<T: DeserializeOwned + std::fmt::Debug>(
        &self,
        doctype: &str,
        name: &str,
    ) -> Result<Option<T>> {
        self.find_doctype_by_name(doctype, name).await
    }

    #[tracing::instrument(skip(self, request))]
    pub async fn list_doctype<T: DeserializeOwned + std::fmt::Debug + Fieldnames>(
        &self,
        request: ListRequest,
    ) -> Result<Vec<T>> {
        const DEFAULT_PAGE_SIZE: usize = 1000;
        const DEFAULT_PAGE_START: usize = 0;

        let ListRequest {
            doctype,
            filters,
            page_size,
            page_start,
            parent,
        } = request;

        let fields =
            serde_json::to_string(T::field_names()).map_err(|source| Error::JsonEncode {
                context: "fields query parameter",
                source,
            })?;
        let filters = serde_json::to_string(&filters).map_err(|source| Error::JsonEncode {
            context: "filters query parameter",
            source,
        })?;

        let mut url = self.resource_url(&doctype)?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("fields", &fields);
            query.append_pair("filters", &filters);
            query.append_pair(
                "limit_page_length",
                &page_size.unwrap_or(DEFAULT_PAGE_SIZE).to_string(),
            );
            query.append_pair(
                "limit_start",
                &page_start.unwrap_or(DEFAULT_PAGE_START).to_string(),
            );

            if let Some(parent) = parent.as_deref() {
                query.append_pair("parent", parent);
            }
        }

        let request = self
            .http
            .get(url)
            .basic_auth(
                &self.settings.key,
                Some(self.settings.secret.expose_secret()),
            )
            .header("Accept", "application/json")
            .build()
            .map_err(|source| Error::RequestBuild { source })?;

        let response = self.log_http_request(request).await?;
        let (status, json, body) = self
            .decode_json_response(response, &doctype, parent.as_deref())
            .await?;

        Self::ensure_erp_success(&doctype, parent.as_deref(), status, &body, &json)?;

        let data = Self::extract_data(&doctype, &json)?;

        let parsed_data: Vec<T> = serde_json::from_value(data.to_owned()).map_err(|source| {
            tracing::error!(
                doctype = %doctype,
                data = %data,
                error = %source,
                "failed parsing data"
            );
            Error::DataDecode {
                doctype: doctype.clone(),
                source,
            }
        })?;

        Ok(parsed_data)
    }

    #[tracing::instrument(skip(self))]
    pub async fn update_doctype<T: Serialize + std::fmt::Debug>(
        &self,
        doctype: &str,
        name: &str,
        data: &T,
    ) -> Result<()> {
        let url = self.resource_url_with_name(doctype, name)?;
        let wrapped = json!({"data":data});
        let request = self
            .http
            .put(url)
            .basic_auth(
                &self.settings.key,
                Some(self.settings.secret.expose_secret()),
            )
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&wrapped)
            .build()
            .map_err(|source| Error::RequestBuild { source })?;

        let response = self.log_http_request(request).await?;
        let (status, json, body) = self.decode_json_response(response, doctype, None).await?;
        Self::ensure_erp_success(doctype, None, status, &body, &json)?;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn insert_doctype<T: Serialize + std::fmt::Debug>(
        &self,
        doctype: &str,
        data: &T,
    ) -> Result<()> {
        let _data: Blank = self.insert_doctype_with_return(doctype, data).await?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn insert_doctype_with_return<
        T: Serialize + std::fmt::Debug,
        R: for<'de> Deserialize<'de>,
    >(
        &self,
        doctype: &str,
        data: &T,
    ) -> Result<R> {
        let wrapped = json!({"data":data});
        let url = self.resource_url(doctype)?;
        let request = self
            .http
            .post(url)
            .basic_auth(
                &self.settings.key,
                Some(self.settings.secret.expose_secret()),
            )
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&wrapped)
            .build()
            .map_err(|source| Error::RequestBuild { source })?;

        let response = self.log_http_request(request).await?;
        let (status, json, body) = self.decode_json_response(response, doctype, None).await?;
        Self::ensure_erp_success(doctype, None, status, &body, &json)?;

        let data = Self::extract_data(doctype, &json)?;
        let parsed: R =
            serde_json::from_value(data.clone()).map_err(|source| Error::DataDecode {
                doctype: doctype.to_string(),
                source,
            })?;

        Ok(parsed)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_sales_pdf(
        &self,
        name: &str,
        format: &str,
        lang: &str,
    ) -> Result<bytes::Bytes> {
        let params = [
            ("doctype", "Sales Invoice"),
            ("name", name),
            ("format", format),
            ("no_letterhead", "0"),
            ("_lang", lang),
        ];

        let url_string = format!(
            "{}/api/method/frappe.utils.print_format.download_pdf",
            self.settings.url
        );
        let url = reqwest::Url::parse_with_params(&url_string, params).map_err(|source| {
            Error::UrlParse {
                url: url_string,
                source,
            }
        })?;

        let request = self
            .http
            .get(url)
            .basic_auth(
                &self.settings.key,
                Some(self.settings.secret.expose_secret()),
            )
            .build()
            .map_err(|source| Error::RequestBuild { source })?;

        let response = self.log_http_request(request).await?;

        let status = response.status();
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let bytes = response
            .bytes()
            .await
            .map_err(|source| Error::Http { source })?;

        if content_type.starts_with("application/json") {
            let body = String::from_utf8_lossy(&bytes).to_string();
            let json: serde_json::Value =
                serde_json::from_slice(&bytes).map_err(|source| Error::JsonDecode {
                    doctype: "Sales Invoice".to_string(),
                    source,
                })?;
            Self::ensure_erp_success("Sales Invoice", None, status, &body, &json)?;
        }

        if !status.is_success() {
            return Err(Error::HttpStatus {
                status,
                doctype: "Sales Invoice".to_string(),
                parent: None,
                body: String::from_utf8_lossy(&bytes).to_string(),
            });
        }

        Ok(bytes)
    }

    fn resource_url(&self, doctype: &str) -> Result<reqwest::Url> {
        let url = format!(
            "{}/api/resource/{}",
            self.settings.url,
            urlencoding::encode(doctype)
        );
        reqwest::Url::parse(&url).map_err(|source| Error::UrlParse { url, source })
    }

    fn resource_url_with_name(&self, doctype: &str, name: &str) -> Result<reqwest::Url> {
        let url = format!(
            "{}/api/resource/{}/{}",
            self.settings.url,
            urlencoding::encode(doctype),
            urlencoding::encode(name)
        );
        reqwest::Url::parse(&url).map_err(|source| Error::UrlParse { url, source })
    }

    async fn decode_json_response(
        &self,
        response: reqwest::Response,
        doctype: &str,
        parent: Option<&str>,
    ) -> Result<(reqwest::StatusCode, serde_json::Value, String)> {
        let status = response.status();
        let bytes = response
            .bytes()
            .await
            .map_err(|source| Error::Http { source })?;
        let body = String::from_utf8_lossy(&bytes).to_string();

        let json = match serde_json::from_slice(&bytes) {
            Ok(json) => json,
            Err(source) => {
                if !status.is_success() {
                    return Err(Error::HttpStatus {
                        status,
                        doctype: doctype.to_string(),
                        parent: parent.map(str::to_string),
                        body,
                    });
                }

                return Err(Error::JsonDecode {
                    doctype: doctype.to_string(),
                    source,
                });
            }
        };

        Ok((status, json, body))
    }

    fn ensure_erp_success(
        doctype: &str,
        parent: Option<&str>,
        status: reqwest::StatusCode,
        body: &str,
        json: &serde_json::Value,
    ) -> Result<()> {
        if let Some(error) = Self::map_erpnext_exception(doctype, parent, json) {
            return Err(error);
        }

        if !status.is_success() {
            return Err(Error::HttpStatus {
                status,
                doctype: doctype.to_string(),
                parent: parent.map(str::to_string),
                body: body.to_string(),
            });
        }

        Ok(())
    }

    fn is_missing_doctype_response(status: reqwest::StatusCode, json: &serde_json::Value) -> bool {
        if json.get("exc_type").and_then(serde_json::Value::as_str) == Some("DoesNotExistError") {
            return true;
        }

        if status != reqwest::StatusCode::NOT_FOUND {
            return false;
        }

        Self::extract_server_messages(json)
            .iter()
            .any(|message| is_missing_message(message))
            || json
                .get("message")
                .and_then(serde_json::Value::as_str)
                .is_some_and(is_missing_message)
    }

    fn extract_server_messages(json: &serde_json::Value) -> Vec<String> {
        let Some(raw_messages) = json
            .get("_server_messages")
            .and_then(serde_json::Value::as_str)
        else {
            return Vec::new();
        };

        let Ok(messages) = serde_json::from_str::<Vec<String>>(raw_messages) else {
            return Vec::new();
        };

        messages
            .into_iter()
            .map(|message| {
                serde_json::from_str::<serde_json::Value>(&message)
                    .ok()
                    .and_then(|value| {
                        value
                            .get("message")
                            .and_then(serde_json::Value::as_str)
                            .map(str::to_string)
                    })
                    .unwrap_or(message)
            })
            .collect()
    }

    fn map_erpnext_exception(
        doctype: &str,
        parent: Option<&str>,
        json: &serde_json::Value,
    ) -> Option<Error> {
        let exception = json.get("exception")?;
        let frappe_exception = exception
            .as_str()
            .map_or_else(|| exception.to_string(), ToString::to_string);

        if is_permission_exception(&frappe_exception) {
            return Some(Error::PermissionDenied {
                doctype: doctype.to_string(),
                parent: parent.map(str::to_string),
                frappe_exception,
            });
        }

        Some(Error::ErpException {
            doctype: doctype.to_string(),
            parent: parent.map(str::to_string),
            frappe_exception,
        })
    }

    fn extract_data<'a>(
        doctype: &str,
        json: &'a serde_json::Value,
    ) -> Result<&'a serde_json::Value> {
        json.get("data").ok_or_else(|| Error::MissingData {
            doctype: doctype.to_string(),
        })
    }

    async fn log_http_request(&self, request: reqwest::Request) -> Result<reqwest::Response> {
        let start = std::time::Instant::now();
        let uri = request.url().clone();

        match self.http.execute(request).await {
            Ok(resp) => {
                let duration = start.elapsed();
                let status = resp.status();
                let size = resp.content_length().unwrap_or(0);
                tracing::debug!(
                    url = %uri,
                    status = %status,
                    content_length = size,
                    duration = %duration.as_millis(),
                    "HTTP request succeeded",
                );
                Ok(resp)
            }
            Err(source) => {
                tracing::error!(
                    "HTTP request to {} failed after {:?}: {}",
                    uri,
                    start.elapsed(),
                    source
                );
                Err(Error::Http { source })
            }
        }
    }
}

fn is_permission_exception(exception: &str) -> bool {
    exception == "frappe.exceptions.PermissionError"
        || exception.starts_with("frappe.exceptions.PermissionError:")
        || exception.contains("frappe.exceptions.PermissionError")
}

fn is_missing_message(message: &str) -> bool {
    let lower = message.to_lowercase();
    lower.contains("nicht gefunden")
        || lower.contains("not found")
        || lower.contains("does not exist")
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct Settings {
    pub url: String,
    pub key: String,
    pub secret: SecretString,
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::Method::GET;
    use httpmock::MockServer;

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct TestDoc {
        name: String,
    }

    fn test_settings(url: String) -> Settings {
        Settings {
            url,
            key: "key".to_string(),
            secret: SecretString::new("secret".to_string().into()),
        }
    }

    #[tokio::test]
    async fn find_doctype_by_name_returns_none_for_404_server_messages() {
        let server = MockServer::start();
        let _mock = server.mock(|when, then| {
            when.method(GET).path("/api/resource/Lead/KID-12983");
            then.status(404)
                .header("content-type", "application/json")
                .body(
                    r#"{"_server_messages":"[\"{\\\"message\\\": \\\"Lead KID-12983 nicht gefunden\\\", \\\"title\\\": \\\"Nachricht\\\", \\\"indicator\\\": \\\"red\\\", \\\"raise_exception\\\": 1}\"]"}"#,
                );
        });

        let client = Client::new(test_settings(server.base_url()));

        let lead = client
            .find_doctype_by_name::<TestDoc>("Lead", "KID-12983")
            .await
            .expect("lookup should succeed");

        assert_eq!(lead, None);
    }

    #[tokio::test]
    async fn get_doctype_by_name_delegates_to_find() {
        let server = MockServer::start();
        let _mock = server.mock(|when, then| {
            when.method(GET).path("/api/resource/Customer/CUST-0001");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"data":{"name":"CUST-0001"}}"#);
        });

        let client = Client::new(test_settings(server.base_url()));

        let customer = client
            .get_doctype_by_name::<TestDoc>("Customer", "CUST-0001")
            .await
            .expect("lookup should succeed");

        assert_eq!(
            customer,
            Some(TestDoc {
                name: "CUST-0001".to_string(),
            })
        );
    }

    #[test]
    fn detects_missing_message_variants() {
        assert!(is_missing_message("Lead KID-1 nicht gefunden"));
        assert!(is_missing_message("Lead KID-1 not found"));
        assert!(is_missing_message("Lead KID-1 does not exist"));
        assert!(!is_missing_message("Permission denied"));
    }

    #[test]
    fn extracts_nested_server_messages() {
        let json = serde_json::json!({
            "_server_messages": "[\"{\\\"message\\\":\\\"Lead KID-1 nicht gefunden\\\"}\"]"
        });

        assert_eq!(
            Client::extract_server_messages(&json),
            vec!["Lead KID-1 nicht gefunden".to_string()]
        );
    }
}
