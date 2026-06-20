//! Check domain availability and read the account balance.
//!
//! Set your credentials in the environment, then run against the sandbox:
//!
//! ```sh
//! NAMECHEAP_API_USER=your-user \
//! NAMECHEAP_API_KEY=your-key \
//! NAMECHEAP_CLIENT_IP=your.whitelisted.ip \
//!   cargo run --example quickstart
//! ```

use namecheap_client::{Client, Environment};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .api_user(std::env::var("NAMECHEAP_API_USER")?)
        .api_key(std::env::var("NAMECHEAP_API_KEY")?)
        .client_ip(std::env::var("NAMECHEAP_CLIENT_IP")?)
        .environment(Environment::Sandbox)
        .build()?;

    let balances = client.users().get_balances().await?;
    println!(
        "Available balance: {:.2} {}",
        balances.available_balance, balances.currency
    );

    let candidates = ["example.com", "a-name-that-is-likely-free-90210.com"];
    println!("\nAvailability:");
    for result in client.domains().check(candidates).await? {
        let status = if result.available {
            "available"
        } else {
            "taken"
        };
        println!("  {:<48} {status}", result.domain);
    }

    Ok(())
}
