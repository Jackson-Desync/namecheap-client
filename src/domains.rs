//! The `namecheap.domains` command namespace: availability checks, registration,
//! and DNS host records.

use serde::Deserialize;

use crate::client::Client;
use crate::error::Result;
use crate::response::{de_bool, de_opt_bool, de_opt_from_str};

/// Accessor for `namecheap.domains.*` commands, returned by
/// [`Client::domains`](crate::Client::domains).
#[derive(Debug, Clone, Copy)]
pub struct Domains<'a> {
    client: &'a Client,
}

impl<'a> Domains<'a> {
    pub(crate) fn new(client: &'a Client) -> Self {
        Self { client }
    }

    /// Checks the availability of one or more domains
    /// (`namecheap.domains.check`).
    ///
    /// Accepts anything iterable of string-like values, so `&["a.com", "b.io"]`,
    /// a `Vec<String>`, and similar all work. The returned vector preserves the
    /// order Namecheap reports, which generally matches the input order.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # async fn run(client: namecheap_client::Client) -> Result<(), namecheap_client::Error> {
    /// for result in client.domains().check(["example.com", "example.io"]).await? {
    ///     println!("{}: available = {}", result.domain, result.available);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an [`Error`](crate::Error) on transport failure, an API error
    /// response, or a decode failure.
    pub async fn check<I, S>(&self, domains: I) -> Result<Vec<DomainCheckResult>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let list = domains
            .into_iter()
            .map(|domain| domain.as_ref().to_owned())
            .collect::<Vec<_>>()
            .join(",");
        let params = vec![("DomainList".to_owned(), list)];
        let payload: CheckPayload = self.client.send("namecheap.domains.check", params).await?;
        Ok(payload.results)
    }

    /// Registers a domain (`namecheap.domains.create`).
    ///
    /// <div class="warning">
    ///
    /// This places a real, billable order against your Namecheap account when
    /// the client targets [`Environment::Production`](crate::Environment::Production).
    /// Develop against [`Environment::Sandbox`](crate::Environment::Sandbox)
    /// first.
    ///
    /// </div>
    ///
    /// # Errors
    ///
    /// Returns an [`Error`](crate::Error) on transport failure, an API error
    /// response (for example insufficient funds or an unsupported TLD), or a
    /// decode failure.
    pub async fn create(&self, request: &DomainCreateRequest) -> Result<DomainCreateResult> {
        let payload: CreatePayload = self
            .client
            .send("namecheap.domains.create", request.to_params())
            .await?;
        Ok(payload.result)
    }

    /// Lists the domains in the account (`namecheap.domains.getList`).
    ///
    /// Returns the first page (up to 100 domains) together with paging totals.
    /// Compare [`DomainListResult::total_items`] with the number of returned
    /// domains to tell whether more pages exist. Each [`DomainListItem`] reports
    /// its [`auto_renew`](DomainListItem::auto_renew) flag and expiry, which is
    /// the read side of managing renewals.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`](crate::Error) on transport failure, an API error
    /// response, or a decode failure.
    pub async fn list(&self) -> Result<DomainListResult> {
        let params = vec![("PageSize".to_owned(), "100".to_owned())];
        let payload: GetListPayload = self
            .client
            .send("namecheap.domains.getList", params)
            .await?;
        Ok(DomainListResult {
            domains: payload.result.domains,
            total_items: payload.paging.total_items,
            current_page: payload.paging.current_page,
            page_size: payload.paging.page_size,
        })
    }

    /// Enables or disables auto-renewal for a domain
    /// (`namecheap.domains.setAutoRenew`).
    ///
    /// Pass the full domain name (for example `"example.com"`). Passing `false`
    /// is how you stop a domain from renewing automatically: it will then lapse
    /// at expiry unless you renew it. Read the current state from
    /// [`list`](Domains::list) via [`DomainListItem::auto_renew`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`](crate::Error) on transport failure, an API error
    /// response, or a decode failure.
    pub async fn set_auto_renew(&self, domain: &str, enabled: bool) -> Result<SetAutoRenewResult> {
        let params = vec![
            ("DomainName".to_owned(), domain.to_owned()),
            (
                "AutoRenew".to_owned(),
                if enabled { "true" } else { "false" }.to_owned(),
            ),
        ];
        let payload: SetAutoRenewPayload = self
            .client
            .send("namecheap.domains.setAutoRenew", params)
            .await?;
        payload
            .into_result()
            .ok_or(crate::error::Error::EmptyResponse)
    }

    /// Accessor for `namecheap.domains.dns.*` commands.
    #[must_use]
    pub fn dns(&self) -> Dns<'a> {
        Dns {
            client: self.client,
        }
    }
}

/// Accessor for `namecheap.domains.dns.*` commands, returned by
/// [`Domains::dns`].
#[derive(Debug, Clone, Copy)]
pub struct Dns<'a> {
    client: &'a Client,
}

impl Dns<'_> {
    /// Reads the current DNS host records for a domain
    /// (`namecheap.domains.dns.getHosts`).
    ///
    /// Pass the domain split into its second-level and top-level parts, the same
    /// way [`SetHostsRequest`] takes them (so `example.com` is `"example"`,
    /// `"com"`). This is the read half of a read-modify-write update: fetch the
    /// records, adjust them via [`GetHostsResult::to_host_records`], and send the
    /// complete set back through [`set_hosts`](Dns::set_hosts).
    ///
    /// [`GetHostsResult::is_using_our_dns`] reports whether the domain points at
    /// Namecheap's DNS; `set_hosts` only takes effect when it does.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`](crate::Error) on transport failure, an API error
    /// response, or a decode failure.
    pub async fn get_hosts(&self, sld: &str, tld: &str) -> Result<GetHostsResult> {
        let params = vec![
            ("SLD".to_owned(), sld.to_owned()),
            ("TLD".to_owned(), tld.to_owned()),
        ];
        let payload: GetHostsPayload = self
            .client
            .send("namecheap.domains.dns.getHosts", params)
            .await?;
        Ok(payload.result)
    }

    /// Replaces the full set of DNS host records for a domain
    /// (`namecheap.domains.dns.setHosts`).
    ///
    /// <div class="warning">
    ///
    /// This command is destructive: it overwrites *all* existing host records
    /// with exactly the records you supply. Any record you omit is deleted. To
    /// change one record, send the complete desired set.
    ///
    /// </div>
    ///
    /// This is the command to use when wiring up email for a domain: supply the
    /// `MX` records and set [`SetHostsRequest::email_type`] to
    /// [`EmailType::Mx`], then add the `TXT` records for SPF, DKIM, and DMARC.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`](crate::Error) on transport failure, an API error
    /// response, or a decode failure.
    pub async fn set_hosts(&self, request: &SetHostsRequest) -> Result<SetHostsResult> {
        let payload: SetHostsPayload = self
            .client
            .send("namecheap.domains.dns.setHosts", request.to_params())
            .await?;
        Ok(payload.result)
    }
}

// --- domains.check ---------------------------------------------------------

#[derive(Debug, Deserialize)]
struct CheckPayload {
    #[serde(rename = "DomainCheckResult", default)]
    results: Vec<DomainCheckResult>,
}

/// The availability result for a single domain from
/// [`Domains::check`].
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct DomainCheckResult {
    /// The domain that was checked.
    #[serde(rename = "@Domain")]
    pub domain: String,
    /// Whether the domain is available to register.
    #[serde(rename = "@Available", deserialize_with = "de_bool")]
    pub available: bool,
    /// Whether Namecheap classifies the domain as a premium name.
    #[serde(rename = "@IsPremiumName", default, deserialize_with = "de_bool")]
    pub is_premium_name: bool,
    /// The premium registration price, when the domain is a premium name.
    #[serde(
        rename = "@PremiumRegistrationPrice",
        default,
        deserialize_with = "de_opt_from_str"
    )]
    pub premium_registration_price: Option<f64>,
    /// A per-domain error code, when Namecheap could not check this entry
    /// (`"0"` indicates no error).
    #[serde(rename = "@ErrorNo", default)]
    pub error_no: Option<String>,
    /// A per-domain description or error message, when present.
    #[serde(rename = "@Description", default)]
    pub description: Option<String>,
}

// --- domains.create --------------------------------------------------------

/// A single postal/email contact used when registering a domain.
///
/// Namecheap requires four contact roles (registrant, technical,
/// administrative, and billing). [`DomainCreateRequest::new`] reuses one
/// `Contact` for all four; build per-role contacts manually if they differ.
///
/// `phone` must use Namecheap's `+NNN.NNNNNNNNNN` format (country code, a dot,
/// then the number), and `country` must be a two-letter ISO country code such
/// as `"US"`.
#[derive(Debug, Clone)]
pub struct Contact {
    /// Given name.
    pub first_name: String,
    /// Family name.
    pub last_name: String,
    /// First address line.
    pub address1: String,
    /// Optional second address line.
    pub address2: Option<String>,
    /// City.
    pub city: String,
    /// State or province.
    pub state_province: String,
    /// Postal or ZIP code.
    pub postal_code: String,
    /// Two-letter ISO country code (for example `"US"`).
    pub country: String,
    /// Phone number in `+NNN.NNNNNNNNNN` format.
    pub phone: String,
    /// Email address.
    pub email_address: String,
    /// Optional organization name.
    pub organization: Option<String>,
}

impl Contact {
    /// Creates a contact from the fields Namecheap requires, leaving the
    /// optional `address2` and `organization` unset.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        first_name: impl Into<String>,
        last_name: impl Into<String>,
        address1: impl Into<String>,
        city: impl Into<String>,
        state_province: impl Into<String>,
        postal_code: impl Into<String>,
        country: impl Into<String>,
        phone: impl Into<String>,
        email_address: impl Into<String>,
    ) -> Self {
        Self {
            first_name: first_name.into(),
            last_name: last_name.into(),
            address1: address1.into(),
            address2: None,
            city: city.into(),
            state_province: state_province.into(),
            postal_code: postal_code.into(),
            country: country.into(),
            phone: phone.into(),
            email_address: email_address.into(),
            organization: None,
        }
    }

    /// Sets the optional second address line.
    #[must_use]
    pub fn with_address2(mut self, address2: impl Into<String>) -> Self {
        self.address2 = Some(address2.into());
        self
    }

    /// Sets the optional organization name.
    #[must_use]
    pub fn with_organization(mut self, organization: impl Into<String>) -> Self {
        self.organization = Some(organization.into());
        self
    }
}

/// A request to register a domain via [`Domains::create`].
#[derive(Debug, Clone)]
pub struct DomainCreateRequest {
    /// The domain to register (for example `"example.com"`).
    pub domain: String,
    /// The number of years to register for.
    pub years: u32,
    /// The registrant contact.
    pub registrant: Contact,
    /// The technical contact.
    pub tech: Contact,
    /// The administrative contact.
    pub admin: Contact,
    /// The billing contact.
    pub aux_billing: Contact,
    /// Custom nameservers. When empty, Namecheap's default DNS is used.
    pub nameservers: Vec<String>,
    /// Whether to request the free WhoisGuard privacy add-on with the order.
    pub add_free_whoisguard: bool,
    /// Whether to enable WhoisGuard privacy once the order completes.
    pub enable_whoisguard: bool,
}

impl DomainCreateRequest {
    /// Builds a registration request that reuses `contact` for all four
    /// required contact roles, with WhoisGuard privacy enabled.
    pub fn new(domain: impl Into<String>, years: u32, contact: Contact) -> Self {
        Self {
            domain: domain.into(),
            years,
            registrant: contact.clone(),
            tech: contact.clone(),
            admin: contact.clone(),
            aux_billing: contact,
            nameservers: Vec::new(),
            add_free_whoisguard: true,
            enable_whoisguard: true,
        }
    }

    /// Sets custom nameservers for the domain.
    #[must_use]
    pub fn with_nameservers<I, S>(mut self, nameservers: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.nameservers = nameservers.into_iter().map(Into::into).collect();
        self
    }

    /// Enables or disables the WhoisGuard privacy add-on (enabled by default).
    #[must_use]
    pub fn with_whois_privacy(mut self, enabled: bool) -> Self {
        self.add_free_whoisguard = enabled;
        self.enable_whoisguard = enabled;
        self
    }

    fn to_params(&self) -> Vec<(String, String)> {
        let mut params = Vec::new();
        params.push(("DomainName".to_owned(), self.domain.clone()));
        params.push(("Years".to_owned(), self.years.to_string()));
        push_contact(&mut params, "Registrant", &self.registrant);
        push_contact(&mut params, "Tech", &self.tech);
        push_contact(&mut params, "Admin", &self.admin);
        push_contact(&mut params, "AuxBilling", &self.aux_billing);
        if !self.nameservers.is_empty() {
            params.push(("Nameservers".to_owned(), self.nameservers.join(",")));
        }
        params.push((
            "AddFreeWhoisguard".to_owned(),
            yes_no(self.add_free_whoisguard),
        ));
        params.push(("WGEnabled".to_owned(), yes_no(self.enable_whoisguard)));
        params
    }
}

fn push_contact(params: &mut Vec<(String, String)>, role: &str, contact: &Contact) {
    let key = |suffix: &str| format!("{role}{suffix}");
    params.push((key("FirstName"), contact.first_name.clone()));
    params.push((key("LastName"), contact.last_name.clone()));
    params.push((key("Address1"), contact.address1.clone()));
    if let Some(address2) = &contact.address2 {
        params.push((key("Address2"), address2.clone()));
    }
    params.push((key("City"), contact.city.clone()));
    params.push((key("StateProvince"), contact.state_province.clone()));
    params.push((key("PostalCode"), contact.postal_code.clone()));
    params.push((key("Country"), contact.country.clone()));
    params.push((key("Phone"), contact.phone.clone()));
    params.push((key("EmailAddress"), contact.email_address.clone()));
    if let Some(organization) = &contact.organization {
        params.push((key("OrganizationName"), organization.clone()));
    }
}

fn yes_no(value: bool) -> String {
    if value { "yes" } else { "no" }.to_owned()
}

#[derive(Debug, Deserialize)]
struct CreatePayload {
    #[serde(rename = "DomainCreateResult")]
    result: DomainCreateResult,
}

/// The outcome of a [`Domains::create`] call.
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct DomainCreateResult {
    /// The domain that was registered.
    #[serde(rename = "@Domain")]
    pub domain: String,
    /// Whether registration succeeded.
    #[serde(rename = "@Registered", deserialize_with = "de_bool")]
    pub registered: bool,
    /// The amount charged for the order, when reported.
    #[serde(
        rename = "@ChargedAmount",
        default,
        deserialize_with = "de_opt_from_str"
    )]
    pub charged_amount: Option<f64>,
    /// The Namecheap internal domain identifier, when reported.
    #[serde(rename = "@DomainID", default, deserialize_with = "de_opt_from_str")]
    pub domain_id: Option<u64>,
    /// The order identifier, when reported.
    #[serde(rename = "@OrderID", default, deserialize_with = "de_opt_from_str")]
    pub order_id: Option<u64>,
    /// The transaction identifier, when reported.
    #[serde(
        rename = "@TransactionID",
        default,
        deserialize_with = "de_opt_from_str"
    )]
    pub transaction_id: Option<u64>,
    /// Whether WhoisGuard privacy was enabled for the domain.
    #[serde(
        rename = "@WhoisguardEnable",
        default,
        deserialize_with = "de_opt_bool"
    )]
    pub whoisguard_enabled: Option<bool>,
    /// Whether the registration is processed out of band rather than in real
    /// time.
    #[serde(
        rename = "@NonRealTimeDomain",
        default,
        deserialize_with = "de_opt_bool"
    )]
    pub non_real_time_domain: Option<bool>,
}

// --- domains.dns.setHosts --------------------------------------------------

/// A DNS record type accepted by [`Dns::set_hosts`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecordType {
    /// IPv4 address record.
    A,
    /// IPv6 address record.
    Aaaa,
    /// Canonical name (alias) record.
    Cname,
    /// Mail exchanger record.
    Mx,
    /// Namecheap "mail forwarding" record type.
    Mxe,
    /// Text record (used for SPF, DKIM, DMARC, verification tokens).
    Txt,
    /// Nameserver record.
    Ns,
    /// Namecheap URL redirect (unmasked) record.
    Url,
    /// Namecheap permanent (301) URL redirect record.
    Url301,
    /// Namecheap masked URL redirect (frame) record.
    Frame,
    /// Certification Authority Authorization record.
    Caa,
}

impl RecordType {
    /// The value Namecheap expects for this record type in the `RecordType`
    /// parameter.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            RecordType::A => "A",
            RecordType::Aaaa => "AAAA",
            RecordType::Cname => "CNAME",
            RecordType::Mx => "MX",
            RecordType::Mxe => "MXE",
            RecordType::Txt => "TXT",
            RecordType::Ns => "NS",
            RecordType::Url => "URL",
            RecordType::Url301 => "URL301",
            RecordType::Frame => "FRAME",
            RecordType::Caa => "CAA",
        }
    }

    /// Parses a record type from the string Namecheap reports (for example in a
    /// [`Dns::get_hosts`] response). Case-insensitive; returns `None` for an
    /// unrecognized type.
    #[must_use]
    pub fn from_api_str(value: &str) -> Option<Self> {
        Some(match value.trim().to_ascii_uppercase().as_str() {
            "A" => RecordType::A,
            "AAAA" => RecordType::Aaaa,
            "CNAME" => RecordType::Cname,
            "MX" => RecordType::Mx,
            "MXE" => RecordType::Mxe,
            "TXT" => RecordType::Txt,
            "NS" => RecordType::Ns,
            "URL" => RecordType::Url,
            "URL301" => RecordType::Url301,
            "FRAME" => RecordType::Frame,
            "CAA" => RecordType::Caa,
            _ => return None,
        })
    }
}

/// The mail setting to apply when calling [`Dns::set_hosts`].
///
/// Namecheap stores a single email routing mode per domain alongside the host
/// records. Set [`EmailType::Mx`] when supplying your own `MX` records (for
/// Google Workspace, Fastmail, and so on).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EmailType {
    /// Custom user-defined `MX` records.
    Mx,
    /// Namecheap email forwarding via `MXE`.
    Mxe,
    /// Namecheap email forwarding.
    Forward,
    /// Namecheap Private Email (Open-Xchange).
    PrivateEmail,
    /// Google Workspace / Gmail preset.
    Gmail,
}

impl EmailType {
    /// The value Namecheap expects for this setting in the `EmailType`
    /// parameter.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            EmailType::Mx => "MX",
            EmailType::Mxe => "MXE",
            EmailType::Forward => "FWD",
            EmailType::PrivateEmail => "OX",
            EmailType::Gmail => "GMAIL",
        }
    }

    /// Parses an email type from the string Namecheap reports (for example in a
    /// [`Dns::get_hosts`] response). Case-insensitive; returns `None` for an
    /// unrecognized value such as `"NONE"`.
    #[must_use]
    pub fn from_api_str(value: &str) -> Option<Self> {
        Some(match value.trim().to_ascii_uppercase().as_str() {
            "MX" => EmailType::Mx,
            "MXE" => EmailType::Mxe,
            "FWD" => EmailType::Forward,
            "OX" => EmailType::PrivateEmail,
            "GMAIL" => EmailType::Gmail,
            _ => return None,
        })
    }
}

/// A single DNS host record to set via [`Dns::set_hosts`].
#[derive(Debug, Clone)]
pub struct HostRecord {
    /// The host (subdomain), for example `"@"` for the apex or `"www"`.
    pub host_name: String,
    /// The record type.
    pub record_type: RecordType,
    /// The record value (an IP, hostname, or text payload).
    pub address: String,
    /// The `MX` preference. Required for `MX` records; ignored otherwise.
    pub mx_pref: Option<u32>,
    /// The record TTL in seconds (Namecheap allows 60 to 60000).
    pub ttl: Option<u32>,
}

impl HostRecord {
    /// Creates a host record with no explicit `MX` preference or TTL.
    pub fn new(
        host_name: impl Into<String>,
        record_type: RecordType,
        address: impl Into<String>,
    ) -> Self {
        Self {
            host_name: host_name.into(),
            record_type,
            address: address.into(),
            mx_pref: None,
            ttl: None,
        }
    }

    /// Creates an `A` (IPv4 address) record.
    pub fn a(host_name: impl Into<String>, ipv4: impl Into<String>) -> Self {
        Self::new(host_name, RecordType::A, ipv4)
    }

    /// Creates an `AAAA` (IPv6 address) record.
    pub fn aaaa(host_name: impl Into<String>, ipv6: impl Into<String>) -> Self {
        Self::new(host_name, RecordType::Aaaa, ipv6)
    }

    /// Creates a `CNAME` (alias) record.
    pub fn cname(host_name: impl Into<String>, target: impl Into<String>) -> Self {
        Self::new(host_name, RecordType::Cname, target)
    }

    /// Creates a `TXT` record (SPF, DKIM, DMARC, verification tokens, ...).
    pub fn txt(host_name: impl Into<String>, value: impl Into<String>) -> Self {
        Self::new(host_name, RecordType::Txt, value)
    }

    /// Creates an `MX` record with the given mail server and preference.
    pub fn mx(
        host_name: impl Into<String>,
        mail_server: impl Into<String>,
        preference: u32,
    ) -> Self {
        Self {
            host_name: host_name.into(),
            record_type: RecordType::Mx,
            address: mail_server.into(),
            mx_pref: Some(preference),
            ttl: None,
        }
    }

    /// Sets the TTL (in seconds) for this record.
    #[must_use]
    pub fn with_ttl(mut self, ttl: u32) -> Self {
        self.ttl = Some(ttl);
        self
    }

    /// Sets the `MX` preference for this record.
    #[must_use]
    pub fn with_mx_pref(mut self, preference: u32) -> Self {
        self.mx_pref = Some(preference);
        self
    }
}

/// A request to replace a domain's DNS host records via [`Dns::set_hosts`].
///
/// Namecheap addresses the domain by its second-level and top-level parts
/// separately, so a domain like `example.com` is `sld = "example"`,
/// `tld = "com"`. Use [`SetHostsRequest::from_domain`] to split a registrable
/// domain automatically.
#[derive(Debug, Clone)]
pub struct SetHostsRequest {
    /// The second-level domain (the `example` of `example.com`).
    pub sld: String,
    /// The top-level domain (the `com` of `example.com`, or `co.uk`).
    pub tld: String,
    /// The complete set of host records the domain should have afterward.
    pub records: Vec<HostRecord>,
    /// The optional email routing mode to apply.
    pub email_type: Option<EmailType>,
}

impl SetHostsRequest {
    /// Builds a request from explicit second-level and top-level domain parts.
    pub fn new(sld: impl Into<String>, tld: impl Into<String>, records: Vec<HostRecord>) -> Self {
        Self {
            sld: sld.into(),
            tld: tld.into(),
            records,
            email_type: None,
        }
    }

    /// Builds a request from a registrable domain, splitting it at the first
    /// dot (so `example.com` and `example.co.uk` both work). Returns `None` if
    /// the input has no dot or an empty part.
    pub fn from_domain(domain: &str, records: Vec<HostRecord>) -> Option<Self> {
        let (sld, tld) = domain.split_once('.')?;
        if sld.is_empty() || tld.is_empty() {
            return None;
        }
        Some(Self::new(sld, tld, records))
    }

    /// Sets the email routing mode (see [`EmailType`]).
    #[must_use]
    pub fn with_email_type(mut self, email_type: EmailType) -> Self {
        self.email_type = Some(email_type);
        self
    }

    fn to_params(&self) -> Vec<(String, String)> {
        let mut params = Vec::new();
        params.push(("SLD".to_owned(), self.sld.clone()));
        params.push(("TLD".to_owned(), self.tld.clone()));
        for (index, record) in self.records.iter().enumerate() {
            let n = index + 1;
            params.push((format!("HostName{n}"), record.host_name.clone()));
            params.push((
                format!("RecordType{n}"),
                record.record_type.as_str().to_owned(),
            ));
            params.push((format!("Address{n}"), record.address.clone()));
            let mx_pref = record.mx_pref.or(match record.record_type {
                RecordType::Mx => Some(10),
                _ => None,
            });
            if let Some(pref) = mx_pref {
                params.push((format!("MXPref{n}"), pref.to_string()));
            }
            if let Some(ttl) = record.ttl {
                params.push((format!("TTL{n}"), ttl.to_string()));
            }
        }
        if let Some(email_type) = self.email_type {
            params.push(("EmailType".to_owned(), email_type.as_str().to_owned()));
        }
        params
    }
}

#[derive(Debug, Deserialize)]
struct SetHostsPayload {
    #[serde(rename = "DomainDNSSetHostsResult")]
    result: SetHostsResult,
}

/// The outcome of a [`Dns::set_hosts`] call.
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct SetHostsResult {
    /// The domain whose records were updated.
    #[serde(rename = "@Domain")]
    pub domain: String,
    /// Whether the update succeeded.
    #[serde(rename = "@IsSuccess", deserialize_with = "de_bool")]
    pub is_success: bool,
}

// --- domains.dns.getHosts --------------------------------------------------

#[derive(Debug, Deserialize)]
struct GetHostsPayload {
    #[serde(rename = "DomainDNSGetHostsResult")]
    result: GetHostsResult,
}

/// The current DNS configuration for a domain, returned by [`Dns::get_hosts`].
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct GetHostsResult {
    /// The domain these records belong to.
    #[serde(rename = "@Domain")]
    pub domain: String,
    /// Whether the domain currently points at Namecheap's DNS. When `false`,
    /// [`Dns::set_hosts`] will not take effect until the nameservers are pointed
    /// back at Namecheap.
    #[serde(rename = "@IsUsingOurDNS", default, deserialize_with = "de_bool")]
    pub is_using_our_dns: bool,
    /// The raw email routing mode reported by the API (for example `"MX"`,
    /// `"MXE"`, `"FWD"`, `"OX"`, `"GMAIL"`, or `"NONE"`). Parse it with
    /// [`EmailType::from_api_str`] if you need the typed form.
    #[serde(rename = "@EmailType", default)]
    pub email_type: Option<String>,
    /// The current host records.
    #[serde(rename = "host", default)]
    pub records: Vec<HostInfo>,
}

impl GetHostsResult {
    /// Converts the current records into [`HostRecord`]s ready to pass back to
    /// [`Dns::set_hosts`], which is the basis of a read-modify-write update.
    ///
    /// Every record type Namecheap's DNS supports is recognized, so in practice
    /// nothing is dropped; a record whose type is somehow unrecognized is
    /// skipped (see [`HostInfo::to_host_record`]).
    #[must_use]
    pub fn to_host_records(&self) -> Vec<HostRecord> {
        self.records
            .iter()
            .filter_map(HostInfo::to_host_record)
            .collect()
    }
}

/// A single DNS host record as returned by [`Dns::get_hosts`].
///
/// This carries the read-only fields the API reports (such as
/// [`host_id`](Self::host_id) and [`is_active`](Self::is_active)). To write a
/// record back, convert it with [`to_host_record`](Self::to_host_record).
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct HostInfo {
    /// Namecheap's internal identifier for this record.
    #[serde(rename = "@HostId", default, deserialize_with = "de_opt_from_str")]
    pub host_id: Option<u64>,
    /// The host (subdomain), for example `"@"` or `"www"`.
    #[serde(rename = "@Name")]
    pub name: String,
    /// The record type as reported by the API (for example `"A"` or `"MX"`).
    #[serde(rename = "@Type")]
    pub record_type: String,
    /// The record value.
    #[serde(rename = "@Address")]
    pub address: String,
    /// The `MX` preference, when applicable.
    #[serde(rename = "@MXPref", default, deserialize_with = "de_opt_from_str")]
    pub mx_pref: Option<u32>,
    /// The record TTL in seconds.
    #[serde(rename = "@TTL", default, deserialize_with = "de_opt_from_str")]
    pub ttl: Option<u32>,
    /// Whether the record is active.
    #[serde(rename = "@IsActive", default, deserialize_with = "de_opt_bool")]
    pub is_active: Option<bool>,
}

impl HostInfo {
    /// Converts this record into a [`HostRecord`] for [`Dns::set_hosts`].
    ///
    /// Returns `None` only if [`record_type`](Self::record_type) is not a type
    /// this crate recognizes (see [`RecordType::from_api_str`]); all standard
    /// Namecheap record types are recognized.
    #[must_use]
    pub fn to_host_record(&self) -> Option<HostRecord> {
        let record_type = RecordType::from_api_str(&self.record_type)?;
        Some(HostRecord {
            host_name: self.name.clone(),
            record_type,
            address: self.address.clone(),
            mx_pref: self.mx_pref,
            ttl: self.ttl,
        })
    }
}

// --- domains.getList -------------------------------------------------------

#[derive(Debug, Deserialize)]
struct GetListPayload {
    #[serde(rename = "DomainGetListResult", default)]
    result: GetListInner,
    #[serde(rename = "Paging", default)]
    paging: Paging,
}

#[derive(Debug, Default, Deserialize)]
struct GetListInner {
    #[serde(rename = "Domain", default)]
    domains: Vec<DomainListItem>,
}

#[derive(Debug, Default, Deserialize)]
struct Paging {
    #[serde(rename = "TotalItems", default)]
    total_items: u32,
    #[serde(rename = "CurrentPage", default)]
    current_page: u32,
    #[serde(rename = "PageSize", default)]
    page_size: u32,
}

/// The result of [`Domains::list`]: the account's domains plus paging totals.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct DomainListResult {
    /// The domains on the returned page.
    pub domains: Vec<DomainListItem>,
    /// The total number of domains in the account, across all pages.
    pub total_items: u32,
    /// The page number that was returned.
    pub current_page: u32,
    /// The page size used for the request.
    pub page_size: u32,
}

/// A single domain entry from [`Domains::list`].
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct DomainListItem {
    /// Namecheap's internal identifier for the domain.
    #[serde(rename = "@ID", default, deserialize_with = "de_opt_from_str")]
    pub id: Option<u64>,
    /// The domain name.
    #[serde(rename = "@Name")]
    pub name: String,
    /// The creation date as reported by the API (for example `"06/20/2026"`).
    #[serde(rename = "@Created", default)]
    pub created: Option<String>,
    /// The expiry date as reported by the API.
    #[serde(rename = "@Expires", default)]
    pub expires: Option<String>,
    /// Whether the domain has expired.
    #[serde(rename = "@IsExpired", default, deserialize_with = "de_opt_bool")]
    pub is_expired: Option<bool>,
    /// Whether the domain is registrar-locked.
    #[serde(rename = "@IsLocked", default, deserialize_with = "de_opt_bool")]
    pub is_locked: Option<bool>,
    /// Whether the domain is set to renew automatically.
    #[serde(rename = "@AutoRenew", default, deserialize_with = "de_bool")]
    pub auto_renew: bool,
    /// The WhoisGuard status (for example `"ENABLED"` or `"NOTPRESENT"`).
    #[serde(rename = "@WhoisGuard", default)]
    pub whois_guard: Option<String>,
    /// Whether the domain currently uses Namecheap's DNS.
    #[serde(rename = "@IsOurDNS", default, deserialize_with = "de_opt_bool")]
    pub is_our_dns: Option<bool>,
}

// --- domains.setAutoRenew --------------------------------------------------

#[derive(Debug, Deserialize)]
struct SetAutoRenewPayload {
    // The live API returns `SetAutoRenewResult`; some references use the
    // `DomainSetAutoRenewResult` spelling, so accept either.
    #[serde(rename = "SetAutoRenewResult", default)]
    result: Option<SetAutoRenewResult>,
    #[serde(rename = "DomainSetAutoRenewResult", default)]
    alt_result: Option<SetAutoRenewResult>,
}

impl SetAutoRenewPayload {
    fn into_result(self) -> Option<SetAutoRenewResult> {
        self.result.or(self.alt_result)
    }
}

/// The result of [`Domains::set_auto_renew`].
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct SetAutoRenewResult {
    /// The domain whose auto-renew setting was changed.
    #[serde(rename = "@Domain")]
    pub domain: String,
    /// Whether the change succeeded.
    #[serde(rename = "@IsSuccess", deserialize_with = "de_bool")]
    pub is_success: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::StatusCode;

    fn value<'a>(params: &'a [(String, String)], key: &str) -> Option<&'a str> {
        params
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.as_str())
    }

    fn sample_contact() -> Contact {
        Contact::new(
            "John",
            "Doe",
            "123 Main St",
            "Los Angeles",
            "CA",
            "90001",
            "US",
            "+1.5555551234",
            "john@example.com",
        )
    }

    #[test]
    fn parses_check_response() {
        let body = r#"<?xml version="1.0" encoding="utf-8"?>
        <ApiResponse Status="OK" xmlns="http://api.namecheap.com/xml.response">
          <Errors />
          <CommandResponse Type="namecheap.domains.check">
            <DomainCheckResult Domain="taken.com" Available="false" ErrorNo="0" Description="" IsPremiumName="false" PremiumRegistrationPrice="0.0000" />
            <DomainCheckResult Domain="free-domain-9876.com" Available="true" ErrorNo="0" Description="" IsPremiumName="false" />
            <DomainCheckResult Domain="fancy.io" Available="true" IsPremiumName="true" PremiumRegistrationPrice="2999.9900" />
          </CommandResponse>
        </ApiResponse>"#;

        let payload: CheckPayload = crate::response::parse(StatusCode::OK, body).unwrap();
        assert_eq!(payload.results.len(), 3);

        assert_eq!(payload.results[0].domain, "taken.com");
        assert!(!payload.results[0].available);

        assert!(payload.results[1].available);
        assert!(!payload.results[1].is_premium_name);

        assert!(payload.results[2].is_premium_name);
        assert_eq!(payload.results[2].premium_registration_price, Some(2999.99));
    }

    #[test]
    fn parses_create_response() {
        let body = r#"<ApiResponse Status="OK" xmlns="http://api.namecheap.com/xml.response">
          <Errors />
          <CommandResponse Type="namecheap.domains.create">
            <DomainCreateResult Domain="example.com" Registered="true" ChargedAmount="10.8700" DomainID="123456" OrderID="654321" TransactionID="111222" WhoisguardEnable="true" NonRealTimeDomain="false" />
          </CommandResponse>
        </ApiResponse>"#;

        let payload: CreatePayload = crate::response::parse(StatusCode::OK, body).unwrap();
        let result = payload.result;
        assert_eq!(result.domain, "example.com");
        assert!(result.registered);
        assert_eq!(result.charged_amount, Some(10.87));
        assert_eq!(result.domain_id, Some(123456));
        assert_eq!(result.order_id, Some(654321));
        assert_eq!(result.whoisguard_enabled, Some(true));
        assert_eq!(result.non_real_time_domain, Some(false));
    }

    #[test]
    fn parses_set_hosts_response() {
        let body = r#"<ApiResponse Status="OK" xmlns="http://api.namecheap.com/xml.response">
          <Errors />
          <CommandResponse Type="namecheap.domains.dns.setHosts">
            <DomainDNSSetHostsResult Domain="example.com" IsSuccess="true" />
          </CommandResponse>
        </ApiResponse>"#;

        let payload: SetHostsPayload = crate::response::parse(StatusCode::OK, body).unwrap();
        assert_eq!(payload.result.domain, "example.com");
        assert!(payload.result.is_success);
    }

    #[test]
    fn create_request_builds_all_contact_roles() {
        let request = DomainCreateRequest::new("example.com", 2, sample_contact())
            .with_nameservers(["ns1.example.com", "ns2.example.com"]);
        let params = request.to_params();

        assert_eq!(value(&params, "DomainName"), Some("example.com"));
        assert_eq!(value(&params, "Years"), Some("2"));
        assert_eq!(
            value(&params, "Nameservers"),
            Some("ns1.example.com,ns2.example.com")
        );
        assert_eq!(value(&params, "AddFreeWhoisguard"), Some("yes"));
        assert_eq!(value(&params, "WGEnabled"), Some("yes"));

        for role in ["Registrant", "Tech", "Admin", "AuxBilling"] {
            assert_eq!(value(&params, &format!("{role}FirstName")), Some("John"));
            assert_eq!(value(&params, &format!("{role}Country")), Some("US"));
            assert_eq!(
                value(&params, &format!("{role}EmailAddress")),
                Some("john@example.com")
            );
        }
    }

    #[test]
    fn create_request_can_disable_privacy() {
        let params = DomainCreateRequest::new("example.com", 1, sample_contact())
            .with_whois_privacy(false)
            .to_params();
        assert_eq!(value(&params, "AddFreeWhoisguard"), Some("no"));
        assert_eq!(value(&params, "WGEnabled"), Some("no"));
    }

    #[test]
    fn set_hosts_request_indexes_records_and_defaults_mx_pref() {
        let request = SetHostsRequest::new(
            "example",
            "com",
            vec![
                HostRecord::mx("@", "mx1.example.com", 10),
                HostRecord::txt("@", "v=spf1 include:_spf.example.com ~all"),
                HostRecord::a("www", "203.0.113.10").with_ttl(300),
            ],
        )
        .with_email_type(EmailType::Mx);
        let params = request.to_params();

        assert_eq!(value(&params, "SLD"), Some("example"));
        assert_eq!(value(&params, "TLD"), Some("com"));

        assert_eq!(value(&params, "HostName1"), Some("@"));
        assert_eq!(value(&params, "RecordType1"), Some("MX"));
        assert_eq!(value(&params, "Address1"), Some("mx1.example.com"));
        assert_eq!(value(&params, "MXPref1"), Some("10"));

        assert_eq!(value(&params, "RecordType2"), Some("TXT"));
        // Non-MX records carry no MX preference.
        assert_eq!(value(&params, "MXPref2"), None);

        assert_eq!(value(&params, "RecordType3"), Some("A"));
        assert_eq!(value(&params, "TTL3"), Some("300"));

        assert_eq!(value(&params, "EmailType"), Some("MX"));
    }

    #[test]
    fn from_domain_splits_at_first_dot() {
        let records = vec![HostRecord::a("@", "203.0.113.10")];

        let simple = SetHostsRequest::from_domain("example.com", records.clone()).unwrap();
        assert_eq!(simple.sld, "example");
        assert_eq!(simple.tld, "com");

        let multi = SetHostsRequest::from_domain("example.co.uk", records.clone()).unwrap();
        assert_eq!(multi.sld, "example");
        assert_eq!(multi.tld, "co.uk");

        assert!(SetHostsRequest::from_domain("nodot", records.clone()).is_none());
        assert!(SetHostsRequest::from_domain(".com", records.clone()).is_none());
        assert!(SetHostsRequest::from_domain("example.", records).is_none());
    }

    #[test]
    fn parses_get_hosts_response_and_round_trips() {
        let body = r#"<ApiResponse Status="OK" xmlns="http://api.namecheap.com/xml.response">
          <Errors />
          <CommandResponse Type="namecheap.domains.dns.getHosts">
            <DomainDNSGetHostsResult Domain="example.com" IsUsingOurDNS="true" EmailType="MX">
              <host HostId="12" Name="@" Type="A" Address="203.0.113.10" MXPref="10" TTL="1800" IsActive="true" IsDDNSEnabled="false" />
              <host HostId="13" Name="@" Type="MX" Address="mx.example.com" MXPref="10" TTL="1800" IsActive="true" />
              <host HostId="14" Name="www" Type="CNAME" Address="example.com." MXPref="10" TTL="60" IsActive="true" />
            </DomainDNSGetHostsResult>
          </CommandResponse>
        </ApiResponse>"#;

        let payload: GetHostsPayload = crate::response::parse(StatusCode::OK, body).unwrap();
        let result = payload.result;
        assert_eq!(result.domain, "example.com");
        assert!(result.is_using_our_dns);
        assert_eq!(result.email_type.as_deref(), Some("MX"));
        assert_eq!(result.records.len(), 3);
        assert_eq!(result.records[0].host_id, Some(12));
        assert_eq!(result.records[0].name, "@");
        assert_eq!(result.records[0].record_type, "A");
        assert_eq!(result.records[0].ttl, Some(1800));
        assert_eq!(result.records[0].is_active, Some(true));

        // Read-modify-write: the records convert straight back to writable form.
        let writable = result.to_host_records();
        assert_eq!(writable.len(), 3);
        assert_eq!(writable[1].record_type, RecordType::Mx);
        assert_eq!(writable[1].mx_pref, Some(10));
        assert_eq!(writable[2].host_name, "www");
        assert_eq!(writable[2].record_type, RecordType::Cname);
    }

    #[test]
    fn record_and_email_types_parse_from_api_strings() {
        for record_type in [
            RecordType::A,
            RecordType::Mx,
            RecordType::Txt,
            RecordType::Caa,
        ] {
            assert_eq!(
                RecordType::from_api_str(record_type.as_str()),
                Some(record_type)
            );
        }
        assert_eq!(RecordType::from_api_str("cname"), Some(RecordType::Cname));
        assert_eq!(RecordType::from_api_str("bogus"), None);

        assert_eq!(EmailType::from_api_str("MX"), Some(EmailType::Mx));
        assert_eq!(EmailType::from_api_str("fwd"), Some(EmailType::Forward));
        assert_eq!(EmailType::from_api_str("NONE"), None);
    }

    #[test]
    fn parses_get_list_response() {
        let body = r#"<ApiResponse Status="OK" xmlns="http://api.namecheap.com/xml.response">
          <Errors />
          <CommandResponse Type="namecheap.domains.getList">
            <DomainGetListResult>
              <Domain ID="1120966" Name="alpha.com" User="someone" Created="06/19/2026" Expires="06/20/2027" IsExpired="false" IsLocked="false" AutoRenew="false" WhoisGuard="NOTPRESENT" IsPremium="false" IsOurDNS="true" />
              <Domain ID="1120967" Name="beta.com" User="someone" Created="06/19/2026" Expires="06/20/2027" IsExpired="false" IsLocked="false" AutoRenew="true" WhoisGuard="ENABLED" IsPremium="false" IsOurDNS="true" />
            </DomainGetListResult>
            <Paging>
              <TotalItems>2</TotalItems>
              <CurrentPage>1</CurrentPage>
              <PageSize>100</PageSize>
            </Paging>
          </CommandResponse>
        </ApiResponse>"#;

        let payload: GetListPayload = crate::response::parse(StatusCode::OK, body).unwrap();
        assert_eq!(payload.paging.total_items, 2);
        assert_eq!(payload.paging.page_size, 100);
        assert_eq!(payload.result.domains.len(), 2);

        let alpha = &payload.result.domains[0];
        assert_eq!(alpha.name, "alpha.com");
        assert_eq!(alpha.id, Some(1120966));
        assert!(!alpha.auto_renew);
        assert_eq!(alpha.expires.as_deref(), Some("06/20/2027"));
        assert_eq!(alpha.is_our_dns, Some(true));

        assert!(payload.result.domains[1].auto_renew);
        assert_eq!(
            payload.result.domains[1].whois_guard.as_deref(),
            Some("ENABLED")
        );
    }

    #[test]
    fn parses_set_auto_renew_response_both_spellings() {
        // The form the live API returned.
        let observed = r#"<ApiResponse Status="OK" xmlns="http://api.namecheap.com/xml.response">
          <Errors />
          <CommandResponse Type="namecheap.domains.setAutoRenew">
            <SetAutoRenewResult Domain="example.com" IsSuccess="true" />
          </CommandResponse>
        </ApiResponse>"#;
        let payload: SetAutoRenewPayload =
            crate::response::parse(StatusCode::OK, observed).unwrap();
        let result = payload.into_result().unwrap();
        assert_eq!(result.domain, "example.com");
        assert!(result.is_success);

        // The alternative element spelling some references use.
        let alternative = r#"<ApiResponse Status="OK" xmlns="http://api.namecheap.com/xml.response">
          <Errors />
          <CommandResponse Type="namecheap.domains.setAutoRenew">
            <DomainSetAutoRenewResult Domain="example.com" IsSuccess="true" />
          </CommandResponse>
        </ApiResponse>"#;
        let payload: SetAutoRenewPayload =
            crate::response::parse(StatusCode::OK, alternative).unwrap();
        assert!(payload.into_result().unwrap().is_success);
    }
}
