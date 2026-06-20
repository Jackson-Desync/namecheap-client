//! Read-modify-write: fetch a domain's DNS records, add one, and write them back.
//!
//! `setHosts` replaces the entire record set, so the safe way to add or change a
//! single record is to read the current set with `getHosts`, modify it, and send
//! the whole thing back.
//!
//! ```sh
//! NAMECHEAP_API_USER=your-user \
//! NAMECHEAP_API_KEY=your-key \
//! NAMECHEAP_CLIENT_IP=your.whitelisted.ip \
//!   cargo run --example dns_update_record
//! ```

use namecheap_client::{Client, Environment, HostRecord, SetHostsRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .api_user(std::env::var("NAMECHEAP_API_USER")?)
        .api_key(std::env::var("NAMECHEAP_API_KEY")?)
        .client_ip(std::env::var("NAMECHEAP_CLIENT_IP")?)
        .environment(Environment::Sandbox)
        .build()?;

    let (sld, tld) = ("example", "com");

    // 1. Read the current records.
    let current = client.domains().dns().get_hosts(sld, tld).await?;
    println!(
        "{} has {} record(s); using Namecheap DNS: {}",
        current.domain,
        current.records.len(),
        current.is_using_our_dns
    );
    for record in &current.records {
        println!(
            "  {:<20} {:<6} {}",
            record.name, record.record_type, record.address
        );
    }

    // 2. Modify: keep everything, then add one TXT record.
    let mut records = current.to_host_records();
    records.push(HostRecord::txt("_acme-challenge", "token-from-your-ca"));

    // 3. Write the complete set back.
    let request = SetHostsRequest::new(sld, tld, records);
    let result = client.domains().dns().set_hosts(&request).await?;
    println!("updated: success = {}", result.is_success);

    Ok(())
}
