//! The asynchronous Namecheap API client and its builder.

use serde::de::DeserializeOwned;

use crate::domains::Domains;
use crate::error::{Error, Result};
use crate::response;
use crate::users::Users;

/// Which Namecheap environment to talk to.
///
/// The sandbox is a free, isolated environment that mirrors the production API.
/// Use it while developing: calls such as
/// [`domains().create()`](crate::domains::Domains::create) place real, billable
/// orders against production. Sandbox accounts are created separately at
/// <https://www.sandbox.namecheap.com>.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Environment {
    /// The live API at `api.namecheap.com`. Commands here have real effects.
    #[default]
    Production,
    /// The sandbox API at `api.sandbox.namecheap.com`. Safe for testing.
    Sandbox,
}

impl Environment {
    /// The base endpoint URL for this environment.
    fn endpoint(self) -> &'static str {
        match self {
            Environment::Production => "https://api.namecheap.com/xml.response",
            Environment::Sandbox => "https://api.sandbox.namecheap.com/xml.response",
        }
    }
}

/// An asynchronous client for the Namecheap API.
///
/// A `Client` bundles your API credentials, the whitelisted client IP, and the
/// chosen [`Environment`], and exposes the supported commands through grouped
/// accessors: [`domains()`](Client::domains) and [`users()`](Client::users).
///
/// The client is cheap to clone and is safe to share across tasks; it wraps a
/// connection-pooling [`reqwest::Client`] internally.
///
/// # Example
///
/// ```no_run
/// # async fn run() -> Result<(), namecheap_client::Error> {
/// use namecheap_client::{Client, Environment};
///
/// let client = Client::builder()
///     .api_user("your-api-user")
///     .api_key("your-api-key")
///     .client_ip("203.0.113.10") // your whitelisted public IPv4
///     .environment(Environment::Sandbox)
///     .build()?;
///
/// let balances = client.users().get_balances().await?;
/// println!("available: {} {}", balances.available_balance, balances.currency);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    api_user: String,
    api_key: String,
    user_name: String,
    client_ip: String,
    environment: Environment,
}

impl Client {
    /// Starts building a client. See [`ClientBuilder`].
    #[must_use]
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// Creates a production client with the minimum required fields.
    ///
    /// `user_name` defaults to `api_user`. To target the sandbox or to set a
    /// distinct username, use [`Client::builder`] instead.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Configuration`] if the underlying HTTP client cannot be
    /// constructed.
    pub fn new(
        api_user: impl Into<String>,
        api_key: impl Into<String>,
        client_ip: impl Into<String>,
    ) -> Result<Self> {
        Self::builder()
            .api_user(api_user)
            .api_key(api_key)
            .client_ip(client_ip)
            .build()
    }

    /// Commands in the `namecheap.domains` namespace.
    #[must_use]
    pub fn domains(&self) -> Domains<'_> {
        Domains::new(self)
    }

    /// Commands in the `namecheap.users` namespace.
    #[must_use]
    pub fn users(&self) -> Users<'_> {
        Users::new(self)
    }

    /// The environment this client targets.
    #[must_use]
    pub fn environment(&self) -> Environment {
        self.environment
    }

    /// Sends a command and decodes the response into the payload type `T`.
    ///
    /// The global parameters (`ApiUser`, `ApiKey`, `UserName`, `ClientIp`, and
    /// `Command`) are added automatically; `params` carries the command-specific
    /// arguments.
    pub(crate) async fn send<T>(&self, command: &str, params: Vec<(String, String)>) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let mut query: Vec<(String, String)> = Vec::with_capacity(params.len() + 5);
        query.push(("ApiUser".to_owned(), self.api_user.clone()));
        query.push(("ApiKey".to_owned(), self.api_key.clone()));
        query.push(("UserName".to_owned(), self.user_name.clone()));
        query.push(("ClientIp".to_owned(), self.client_ip.clone()));
        query.push(("Command".to_owned(), command.to_owned()));
        query.extend(params);

        let response = self
            .http
            .get(self.environment.endpoint())
            .query(&query)
            .send()
            .await?;

        let status = response.status();
        // Decode as UTF-8 (Namecheap responses are UTF-8). Using `bytes()` keeps
        // the dependency surface small by avoiding reqwest's `charset` feature.
        let bytes = response.bytes().await?;
        let body = String::from_utf8_lossy(&bytes);
        response::parse(status, &body)
    }
}

/// A builder for [`Client`].
///
/// Construct one with [`Client::builder`], set the credentials and the
/// whitelisted client IP, then call [`build`](ClientBuilder::build).
#[derive(Debug, Default, Clone)]
pub struct ClientBuilder {
    api_user: Option<String>,
    api_key: Option<String>,
    user_name: Option<String>,
    client_ip: Option<String>,
    environment: Environment,
    http: Option<reqwest::Client>,
}

impl ClientBuilder {
    /// Sets the API username (the `ApiUser` parameter).
    #[must_use]
    pub fn api_user(mut self, value: impl Into<String>) -> Self {
        self.api_user = Some(value.into());
        self
    }

    /// Sets the API key (the `ApiKey` parameter).
    #[must_use]
    pub fn api_key(mut self, value: impl Into<String>) -> Self {
        self.api_key = Some(value.into());
        self
    }

    /// Sets the account username (the `UserName` parameter).
    ///
    /// When omitted, this defaults to the value of [`api_user`](Self::api_user).
    /// It only differs when acting on behalf of another account.
    #[must_use]
    pub fn user_name(mut self, value: impl Into<String>) -> Self {
        self.user_name = Some(value.into());
        self
    }

    /// Sets the whitelisted client IP (the `ClientIp` parameter).
    ///
    /// This must be the public IPv4 address you have whitelisted in the
    /// Namecheap API settings, and it must match the address the request
    /// actually originates from.
    #[must_use]
    pub fn client_ip(mut self, value: impl Into<String>) -> Self {
        self.client_ip = Some(value.into());
        self
    }

    /// Selects the [`Environment`]. Defaults to [`Environment::Production`].
    #[must_use]
    pub fn environment(mut self, environment: Environment) -> Self {
        self.environment = environment;
        self
    }

    /// Supplies a preconfigured [`reqwest::Client`] (custom timeouts, proxy, and
    /// so on). When omitted, a default client is created.
    #[must_use]
    pub fn http_client(mut self, client: reqwest::Client) -> Self {
        self.http = Some(client);
        self
    }

    /// Builds the [`Client`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::Configuration`] if `api_user`, `api_key`, or `client_ip`
    /// is missing, or if a default HTTP client could not be constructed.
    pub fn build(self) -> Result<Client> {
        let api_user = self
            .api_user
            .ok_or_else(|| Error::Configuration("`api_user` is required".to_owned()))?;
        let api_key = self
            .api_key
            .ok_or_else(|| Error::Configuration("`api_key` is required".to_owned()))?;
        let client_ip = self
            .client_ip
            .ok_or_else(|| Error::Configuration("`client_ip` is required".to_owned()))?;
        let user_name = self.user_name.unwrap_or_else(|| api_user.clone());

        let http = match self.http {
            Some(client) => client,
            None => reqwest::Client::builder()
                .user_agent(concat!("namecheap-client/", env!("CARGO_PKG_VERSION")))
                .build()
                .map_err(|error| Error::Configuration(error.to_string()))?,
        };

        Ok(Client {
            http,
            api_user,
            api_key,
            user_name,
            client_ip,
            environment: self.environment,
        })
    }
}
