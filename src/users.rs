//! The `namecheap.users` command namespace.

use serde::Deserialize;

use crate::client::Client;
use crate::error::Result;

/// Accessor for `namecheap.users.*` commands, returned by
/// [`Client::users`](crate::Client::users).
#[derive(Debug, Clone, Copy)]
pub struct Users<'a> {
    client: &'a Client,
}

impl<'a> Users<'a> {
    pub(crate) fn new(client: &'a Client) -> Self {
        Self { client }
    }

    /// Returns the account balances (`namecheap.users.getBalances`).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # async fn run(client: namecheap_client::Client) -> Result<(), namecheap_client::Error> {
    /// let balances = client.users().get_balances().await?;
    /// println!("{} {}", balances.available_balance, balances.currency);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an [`Error`](crate::Error) on transport failure, an API error
    /// response, or a decode failure.
    pub async fn get_balances(&self) -> Result<Balances> {
        let payload: BalancesPayload = self
            .client
            .send("namecheap.users.getBalances", Vec::new())
            .await?;
        Ok(payload.result)
    }
}

#[derive(Debug, Deserialize)]
struct BalancesPayload {
    #[serde(rename = "UserGetBalancesResult")]
    result: Balances,
}

/// Account balances returned by [`Users::get_balances`].
///
/// Amounts are parsed into [`f64`]. Because the API returns them as decimal
/// strings, treat these values as display figures rather than as inputs to
/// exact financial arithmetic.
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct Balances {
    /// The account currency (for example `"USD"`).
    #[serde(rename = "@Currency")]
    pub currency: String,
    /// The balance available to spend.
    #[serde(rename = "@AvailableBalance")]
    pub available_balance: f64,
    /// The total account balance.
    #[serde(rename = "@AccountBalance")]
    pub account_balance: f64,
    /// The amount earned (for reseller/affiliate accounts).
    #[serde(rename = "@EarnedAmount")]
    pub earned_amount: f64,
    /// The amount that can currently be withdrawn.
    #[serde(rename = "@WithdrawableAmount")]
    pub withdrawable_amount: f64,
    /// The funds required to cover upcoming auto-renewals.
    #[serde(rename = "@FundsRequiredForAutoRenew")]
    pub funds_required_for_auto_renew: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::StatusCode;

    #[test]
    fn parses_balances_response() {
        let body = r#"<?xml version="1.0" encoding="utf-8"?>
        <ApiResponse Status="OK" xmlns="http://api.namecheap.com/xml.response">
          <Errors />
          <CommandResponse Type="namecheap.users.getBalances">
            <UserGetBalancesResult Currency="USD" AvailableBalance="4932.96" AccountBalance="4932.96" EarnedAmount="381.30" WithdrawableAmount="1500.00" FundsRequiredForAutoRenew="0.00" />
          </CommandResponse>
        </ApiResponse>"#;

        let payload: BalancesPayload = crate::response::parse(StatusCode::OK, body).unwrap();
        let balances = payload.result;
        assert_eq!(balances.currency, "USD");
        assert_eq!(balances.available_balance, 4932.96);
        assert_eq!(balances.earned_amount, 381.30);
        assert_eq!(balances.funds_required_for_auto_renew, 0.0);
    }
}
