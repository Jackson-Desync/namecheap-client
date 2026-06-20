//! An unofficial, async Rust client for the [Namecheap](https://www.namecheap.com)
//! API.
//!
//! This crate wraps the parts of the Namecheap API needed to check, register,
//! and configure domains, so you do not have to hand-roll HTTP requests and XML
//! parsing. It is **not** affiliated with or endorsed by Namecheap.
//!
//! # Supported commands
//!
//! - [`Client::domains().check()`](domains::Domains::check) -
//!   `namecheap.domains.check`
//! - [`Client::domains().create()`](domains::Domains::create) -
//!   `namecheap.domains.create`
//! - [`Client::domains().dns().get_hosts()`](domains::Dns::get_hosts) -
//!   `namecheap.domains.dns.getHosts`
//! - [`Client::domains().dns().set_hosts()`](domains::Dns::set_hosts) -
//!   `namecheap.domains.dns.setHosts`
//! - [`Client::users().get_balances()`](users::Users::get_balances) -
//!   `namecheap.users.getBalances`
//!
//! # Requirements
//!
//! Every request is authenticated with your API user, API key, and account
//! username, and must originate from an IP address you have whitelisted in the
//! Namecheap API settings. That same address is sent as the `ClientIp`
//! parameter, so it must match your real outbound public IPv4 address.
//!
//! # Example
//!
//! ```no_run
//! use namecheap_client::{Client, Environment};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), namecheap_client::Error> {
//!     let client = Client::builder()
//!         .api_user("your-api-user")
//!         .api_key("your-api-key")
//!         .client_ip("203.0.113.10") // your whitelisted public IPv4
//!         .environment(Environment::Sandbox)
//!         .build()?;
//!
//!     for result in client.domains().check(["example.com", "rust-lang.org"]).await? {
//!         println!("{}: available = {}", result.domain, result.available);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! # TLS backends
//!
//! By default the crate uses [`rustls`](https://github.com/rustls/rustls) so it
//! builds without a system OpenSSL installation. To use the platform-native TLS
//! stack instead, disable default features and enable `native-tls`:
//!
//! ```toml
//! namecheap-client = { version = "0.1", default-features = false, features = ["native-tls"] }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

mod client;
mod error;
mod response;

pub mod domains;
pub mod users;

pub use client::{Client, ClientBuilder, Environment};
pub use error::{ApiError, ApiErrorEntry, Error, Result};

pub use domains::{
    Contact, Dns, DomainCheckResult, DomainCreateRequest, DomainCreateResult, Domains, EmailType,
    GetHostsResult, HostInfo, HostRecord, RecordType, SetHostsRequest, SetHostsResult,
};
pub use users::{Balances, Users};
