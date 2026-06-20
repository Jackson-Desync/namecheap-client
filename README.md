# namecheap-client

An unofficial, async Rust client for the [Namecheap](https://www.namecheap.com) API.

[![CI](https://github.com/Jackson-Desync/namecheap-client/actions/workflows/ci.yml/badge.svg)](https://github.com/Jackson-Desync/namecheap-client/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/namecheap-client.svg)](https://crates.io/crates/namecheap-client)
[![docs.rs](https://img.shields.io/docsrs/namecheap-client)](https://docs.rs/namecheap-client)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

`namecheap-client` is a small, type-safe wrapper over the Namecheap API. It takes
care of the HTTPS requests and the XML parsing, so you can check, register, and
configure domains using ordinary Rust types and `async`/`await` instead of
hand-rolling query strings and walking an XML document.

> **Disclaimer:** This is an independent, community project. It is not affiliated
> with, endorsed by, or sponsored by Namecheap, and it does not use Namecheap's
> trademarks or branding. It is maintained on a best-effort basis. "Namecheap" is
> a trademark of its respective owner.

## Scope

Version 0.1 focuses on the core commands needed to stand a domain up end to end,
including pointing its email at a provider:

| Method | Namecheap command | Purpose |
| --- | --- | --- |
| `client.domains().check(...)` | `namecheap.domains.check` | Check domain availability |
| `client.domains().create(...)` | `namecheap.domains.create` | Register a domain |
| `client.domains().list()` | `namecheap.domains.getList` | List domains (with auto-renew status) |
| `client.domains().set_auto_renew(...)` | `namecheap.domains.setAutoRenew` | Enable or disable auto-renewal |
| `client.domains().dns().get_hosts(...)` | `namecheap.domains.dns.getHosts` | Read DNS host records |
| `client.domains().dns().set_hosts(...)` | `namecheap.domains.dns.setHosts` | Replace DNS host records |
| `client.users().get_balances()` | `namecheap.users.getBalances` | Read account balances |

Not yet covered: full endpoint coverage, a blocking (synchronous) API, and
reseller commands. The surface area is intentionally small. If a command you need
is missing, please open an issue.

## Installation

Add the crate and an async runtime to your `Cargo.toml`:

```toml
[dependencies]
namecheap-client = "0.1"
tokio = { version = "1", features = ["full"] }
```

This crate has not been published to crates.io yet. Until the first release is
out, depend on it directly from Git:

```toml
[dependencies]
namecheap-client = { git = "https://github.com/Jackson-Desync/namecheap-client" }
```

## Getting API access

Before any call will succeed you need three things from Namecheap:

1. **API access enabled** on your account (Profile, then Tools, then the
   Namecheap API Access section).
2. **Your public IPv4 address whitelisted** in that same section.
3. **That exact IP used as `client_ip`.** Namecheap requires the request to come
   from a whitelisted address, and the address you send as `ClientIp` has to
   match the address the request actually originates from.

While developing, point the client at the sandbox (`Environment::Sandbox`). The
sandbox uses separate credentials and a separate account that you create at
[sandbox.namecheap.com](https://www.sandbox.namecheap.com). Calls such as domain
registration place real, billable orders in production.

## Quick start

```rust
use namecheap_client::{Client, Environment};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .api_user("your-api-user")
        .api_key("your-api-key")
        .client_ip("203.0.113.10") // your whitelisted public IPv4
        .environment(Environment::Sandbox)
        .build()?;

    // Account balance.
    let balances = client.users().get_balances().await?;
    println!("Available: {:.2} {}", balances.available_balance, balances.currency);

    // Availability.
    for result in client.domains().check(["example.com", "example.io"]).await? {
        let status = if result.available { "available" } else { "taken" };
        println!("{}: {status}", result.domain);
    }

    Ok(())
}
```

`UserName` defaults to `api_user`. Set it explicitly with `.user_name(...)` only
when you act on behalf of a different account.

## Setting up email DNS (MX, SPF, DKIM, DMARC)

`set_hosts` is the command for wiring a domain up to an email provider. Supply
the `MX` records and set the email type to `EmailType::Mx`, then add the `TXT`
records for SPF, DKIM, and DMARC.

> **Important:** `setHosts` replaces the full set of host records. Any record you
> do not include is removed. Always send the complete set you want the domain to
> end up with, including your website records.

```rust
use namecheap_client::{EmailType, HostRecord, SetHostsRequest};

// `client` is a built Client (see Quick start above).
let records = vec![
    // Keep the website pointing somewhere.
    HostRecord::a("@", "203.0.113.10"),
    HostRecord::cname("www", "example.com."),
    // Mail exchangers (Google Workspace shown here).
    HostRecord::mx("@", "aspmx.l.google.com", 1),
    HostRecord::mx("@", "alt1.aspmx.l.google.com", 5),
    // SPF, DKIM, and DMARC.
    HostRecord::txt("@", "v=spf1 include:_spf.google.com ~all"),
    HostRecord::txt("google._domainkey", "v=DKIM1; k=rsa; p=MIGf..."),
    HostRecord::txt("_dmarc", "v=DMARC1; p=quarantine; rua=mailto:dmarc@example.com"),
];

let request = SetHostsRequest::from_domain("example.com", records)
    .expect("registrable domain")
    .with_email_type(EmailType::Mx);

let result = client.domains().dns().set_hosts(&request).await?;
println!("success = {}", result.is_success);
```

`from_domain` splits a registrable domain at the first dot, which is correct for
both `example.com` and `example.co.uk`. If you already have the parts, construct
the request with `SetHostsRequest::new(sld, tld, records)`.

## Reading and updating records

Because `setHosts` replaces everything, the safe way to change a single record is
read-modify-write: read the current records with `get_hosts`, change what you
need, and send the full set back. `GetHostsResult::to_host_records` converts the
records straight into the writable form `set_hosts` expects.

```rust
use namecheap_client::{HostRecord, SetHostsRequest};

// `client` is a built Client (see Quick start above).
let (sld, tld) = ("example", "com");

// Read.
let current = client.domains().dns().get_hosts(sld, tld).await?;
for record in &current.records {
    println!("{} {} {}", record.name, record.record_type, record.address);
}

// Modify: keep everything, add one record.
let mut records = current.to_host_records();
records.push(HostRecord::txt("_acme-challenge", "token-from-your-ca"));

// Write the complete set back.
let request = SetHostsRequest::new(sld, tld, records);
client.domains().dns().set_hosts(&request).await?;
```

`GetHostsResult::is_using_our_dns` tells you whether the domain points at
Namecheap's DNS; `set_hosts` only takes effect when it does.

## Registering a domain

> **Heads up:** In production, `create` places a real order and charges your
> account. Test against the sandbox first.

```rust
use namecheap_client::{Contact, DomainCreateRequest};

// `client` is a built Client (see Quick start above).
let contact = Contact::new(
    "John", "Doe",
    "123 Example St",
    "Los Angeles", "CA", "90001", "US",
    "+1.5555551234",            // Namecheap phone format: +NNN.NNNNNNNNNN
    "john@example.com",
);

// One contact is reused for the registrant, tech, admin, and billing roles.
let request = DomainCreateRequest::new("example.com", 1, contact);

let result = client.domains().create(&request).await?;
println!("registered = {}, charged = {:?}", result.registered, result.charged_amount);
```

WhoisGuard privacy is requested by default; disable it with
`.with_whois_privacy(false)`. Set custom nameservers with `.with_nameservers([...])`.

## Listing domains and auto-renewal

List the domains on the account (with each one's auto-renew flag and expiry), and
turn auto-renewal on or off:

```rust
// `client` is a built Client (see Quick start above).
let list = client.domains().list().await?;
for domain in &list.domains {
    println!("{}: auto_renew={} expires={:?}", domain.name, domain.auto_renew, domain.expires);
}

// Stop a domain from renewing automatically (it then lapses at expiry).
client.domains().set_auto_renew("example.com", false).await?;
```

Domains registered through the API default to auto-renew off, so a domain will
not renew on its own unless you enable it.

## Error handling

Failures are split into distinct categories so you can react to them precisely.
Errors that Namecheap reports inside the XML body are surfaced as `Error::Api`,
separate from transport-level HTTP failures.

```rust
use namecheap_client::Error;

// `client` is a built Client (see Quick start above).
match client.domains().check(["example.com"]).await {
    Ok(results) => { /* ... */ }
    Err(Error::Api(api)) => {
        // Branch on Namecheap's documented error numbers.
        if api.has_code("2030280") {
            eprintln!("that TLD is not supported");
        } else {
            eprintln!("API error: {api}");
        }
    }
    Err(Error::Http(err)) => eprintln!("network/transport error: {err}"),
    Err(other) => eprintln!("error: {other}"),
}
```

## TLS backends

By default the crate uses [rustls](https://github.com/rustls/rustls), so it
builds without a system OpenSSL installation. To use the platform-native TLS
stack instead, disable default features and enable `native-tls`:

```toml
[dependencies]
namecheap-client = { version = "0.1", default-features = false, features = ["native-tls"] }
```

## Running the examples

The repository includes runnable examples that read credentials from the
environment:

```sh
NAMECHEAP_API_USER=your-user \
NAMECHEAP_API_KEY=your-key \
NAMECHEAP_CLIENT_IP=your.whitelisted.ip \
  cargo run --example quickstart

# and the email DNS walkthrough:
  cargo run --example dns_email_setup
```

## Design notes

- Async-first, built on [reqwest](https://docs.rs/reqwest) and
  [tokio](https://tokio.rs).
- Strongly-typed request and response structs for every supported command.
- XML decoding with [serde](https://serde.rs) and
  [quick-xml](https://docs.rs/quick-xml).
- `#![forbid(unsafe_code)]` and no `build.rs` in this crate, to keep it easy to
  audit.
- A deliberately small dependency set.

## Versioning

This project follows [Semantic Versioning](https://semver.org). While the major
version is `0`, minor releases may contain breaking changes. A recent stable
Rust toolchain is recommended.

## Contributing

Issues and pull requests are welcome. For larger changes, please open an issue
first to discuss the direction. By contributing you agree that your work is
licensed under the same terms as the project.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
