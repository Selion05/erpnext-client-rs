# Erpnext Client

An async Rust client for interacting with [ERPNext](https://erpnext.com/) via their API.

Supports reading, inserting, and updating doctypes using basic authentication.

Use `find_doctype_by_name()` when a missing document is an expected outcome.

## Todo
- search doctypes with filter
## 🚀 Usage

```rust
use erpnext_client::{Client, Settings};
use secrecy::SecretString;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Customer {
    name: String,
    customer_name: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = Settings {
        url: "https://example.com".into(),
        key: "your-api-key".into(),
        secret: SecretString::new("your-secret-key".into()),
    };

    let client = Client::new(settings);

    if let Some(customer) = client
        .find_doctype_by_name::<Customer>("Customer", "CUST-0001")
        .await?
    {
        println!("Customer: {:?}", customer);
    }

    Ok(())
}

```
