//! Configure a domain's DNS host records for email (MX, SPF, DKIM, DMARC).
//!
//! `setHosts` replaces the entire set of host records, so this example sends
//! every record the domain should have, including the website records. Run it
//! against the sandbox first:
//!
//! ```sh
//! NAMECHEAP_API_USER=your-user \
//! NAMECHEAP_API_KEY=your-key \
//! NAMECHEAP_CLIENT_IP=your.whitelisted.ip \
//!   cargo run --example dns_email_setup
//! ```

use namecheap_client::{Client, EmailType, Environment, HostRecord, SetHostsRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .api_user(std::env::var("NAMECHEAP_API_USER")?)
        .api_key(std::env::var("NAMECHEAP_API_KEY")?)
        .client_ip(std::env::var("NAMECHEAP_CLIENT_IP")?)
        .environment(Environment::Sandbox)
        .build()?;

    let records = vec![
        // Website records (kept so they are not removed by the replace).
        HostRecord::a("@", "203.0.113.10"),
        HostRecord::cname("www", "example.com."),
        // Mail exchangers (Google Workspace shown as an example).
        HostRecord::mx("@", "aspmx.l.google.com", 1),
        HostRecord::mx("@", "alt1.aspmx.l.google.com", 5),
        // SPF, DKIM, and DMARC.
        HostRecord::txt("@", "v=spf1 include:_spf.google.com ~all"),
        HostRecord::txt("google._domainkey", "v=DKIM1; k=rsa; p=MIGfMA0GCSq..."),
        HostRecord::txt(
            "_dmarc",
            "v=DMARC1; p=quarantine; rua=mailto:dmarc@example.com",
        ),
    ];

    let request = SetHostsRequest::from_domain("example.com", records)
        .expect("domain must contain a second-level and top-level part")
        .with_email_type(EmailType::Mx);

    let result = client.domains().dns().set_hosts(&request).await?;
    println!("Updated {}: success = {}", result.domain, result.is_success);

    Ok(())
}
