//! The `namecheap.users` command namespace.

use serde::Deserialize;

use crate::client::Client;
use crate::error::Result;
use crate::response::de_opt_from_str;

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

    /// Looks up product pricing (`namecheap.users.getPricing`).
    ///
    /// This is a free, read-only call. For domain prices, build the request with
    /// [`PricingRequest::domains`] and narrow it by action and TLD. The response
    /// is nested (product type, then category, then product, then per-duration
    /// prices); use [`PricingResult::prices`] to flatten it to the price entries.
    ///
    /// Note this returns *standard* TLD pricing. Per-name premium prices come
    /// from [`domains().check()`](crate::domains::Domains::check) instead.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # async fn run(client: namecheap_client::Client) -> Result<(), namecheap_client::Error> {
    /// use namecheap_client::PricingRequest;
    /// let result = client
    ///     .users()
    ///     .get_pricing(&PricingRequest::domains().action("REGISTER").tld("com"))
    ///     .await?;
    /// for price in result.prices() {
    ///     println!("{} {}: {} {}", price.duration, price.duration_type, price.price, price.currency);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an [`Error`](crate::Error) on transport failure, an API error
    /// response, or a decode failure.
    pub async fn get_pricing(&self, request: &PricingRequest) -> Result<PricingResult> {
        let payload: GetPricingPayload = self
            .client
            .send("namecheap.users.getPricing", request.to_params())
            .await?;
        Ok(PricingResult {
            product_types: payload.result.product_types,
        })
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

/// A request for [`Users::get_pricing`].
///
/// For domain prices, start from [`PricingRequest::domains`] and narrow with
/// [`action`](Self::action) and [`tld`](Self::tld).
#[derive(Debug, Clone)]
pub struct PricingRequest {
    /// The product type (`"DOMAIN"`, `"SSLCERTIFICATE"`, or `"WHOISGUARD"`).
    pub product_type: String,
    /// The product category (for domains this is `"DOMAINS"`).
    pub product_category: Option<String>,
    /// The action to price (`"REGISTER"`, `"RENEW"`, `"TRANSFER"`, `"REACTIVATE"`).
    /// When omitted, all actions are returned.
    pub action_name: Option<String>,
    /// The specific product, for example a TLD like `"com"`. When omitted, all
    /// products are returned (which can be a large response).
    pub product_name: Option<String>,
    /// An optional promotion code to apply.
    pub promotion_code: Option<String>,
}

impl PricingRequest {
    /// A request for domain pricing (`ProductType=DOMAIN`).
    #[must_use]
    pub fn domains() -> Self {
        Self {
            product_type: "DOMAIN".to_owned(),
            product_category: Some("DOMAINS".to_owned()),
            action_name: None,
            product_name: None,
            promotion_code: None,
        }
    }

    /// A request for an arbitrary product type.
    pub fn new(product_type: impl Into<String>) -> Self {
        Self {
            product_type: product_type.into(),
            product_category: None,
            action_name: None,
            product_name: None,
            promotion_code: None,
        }
    }

    /// Narrows to a single action (`"REGISTER"`, `"RENEW"`, ...).
    #[must_use]
    pub fn action(mut self, action: impl Into<String>) -> Self {
        self.action_name = Some(action.into());
        self
    }

    /// Narrows to a single TLD or product (for example `"com"`).
    #[must_use]
    pub fn tld(mut self, tld: impl Into<String>) -> Self {
        self.product_name = Some(tld.into());
        self
    }

    /// Sets the product category explicitly.
    #[must_use]
    pub fn category(mut self, category: impl Into<String>) -> Self {
        self.product_category = Some(category.into());
        self
    }

    /// Applies a promotion code.
    #[must_use]
    pub fn promotion_code(mut self, code: impl Into<String>) -> Self {
        self.promotion_code = Some(code.into());
        self
    }

    fn to_params(&self) -> Vec<(String, String)> {
        let mut params = vec![("ProductType".to_owned(), self.product_type.clone())];
        if let Some(category) = &self.product_category {
            params.push(("ProductCategory".to_owned(), category.clone()));
        }
        if let Some(action) = &self.action_name {
            params.push(("ActionName".to_owned(), action.clone()));
        }
        if let Some(product) = &self.product_name {
            params.push(("ProductName".to_owned(), product.clone()));
        }
        if let Some(code) = &self.promotion_code {
            params.push(("PromotionCode".to_owned(), code.clone()));
        }
        params
    }
}

#[derive(Debug, Deserialize)]
struct GetPricingPayload {
    #[serde(rename = "UserGetPricingResult", default)]
    result: UserGetPricingResult,
}

#[derive(Debug, Default, Deserialize)]
struct UserGetPricingResult {
    #[serde(rename = "ProductType", default)]
    product_types: Vec<ProductTypePricing>,
}

/// Pricing returned by [`Users::get_pricing`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct PricingResult {
    /// The product types in the response (for a narrowed domain query, usually
    /// just one).
    pub product_types: Vec<ProductTypePricing>,
}

impl PricingResult {
    /// Flattens the nested response into the individual [`Price`] entries, which
    /// is usually what you want once the request is narrowed to one TLD and
    /// action (you get one entry per available duration).
    #[must_use]
    pub fn prices(&self) -> Vec<&Price> {
        self.product_types
            .iter()
            .flat_map(|product_type| &product_type.categories)
            .flat_map(|category| &category.products)
            .flat_map(|product| &product.prices)
            .collect()
    }
}

/// Pricing grouped under a product type (for example `"domains"`).
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct ProductTypePricing {
    /// The product type name.
    #[serde(rename = "@Name")]
    pub name: String,
    /// The categories under this product type.
    #[serde(rename = "ProductCategory", default)]
    pub categories: Vec<ProductCategoryPricing>,
}

/// Pricing grouped under a category (for example `"register"` or `"renew"`).
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct ProductCategoryPricing {
    /// The category name.
    #[serde(rename = "@Name")]
    pub name: String,
    /// The products under this category.
    #[serde(rename = "Product", default)]
    pub products: Vec<ProductPricing>,
}

/// Pricing for a single product (for domains, a TLD such as `"com"`).
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct ProductPricing {
    /// The product name (a TLD, for domain pricing).
    #[serde(rename = "@Name")]
    pub name: String,
    /// The price for each available duration.
    #[serde(rename = "Price", default)]
    pub prices: Vec<Price>,
}

/// A single price entry for one duration of one product.
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct Price {
    /// The number of duration units (for example `1`).
    #[serde(rename = "@Duration")]
    pub duration: u32,
    /// The duration unit (for example `"YEAR"`).
    #[serde(rename = "@DurationType")]
    pub duration_type: String,
    /// The price for this duration.
    #[serde(rename = "@Price")]
    pub price: f64,
    /// The standard (list) price.
    #[serde(rename = "@RegularPrice")]
    pub regular_price: f64,
    /// The price for your account (may reflect account-specific rates).
    #[serde(rename = "@YourPrice")]
    pub your_price: f64,
    /// The promotional price, when a promotion applies.
    #[serde(
        rename = "@PromotionPrice",
        default,
        deserialize_with = "de_opt_from_str"
    )]
    pub promotion_price: Option<f64>,
    /// Any additional cost, such as the ICANN fee.
    #[serde(
        rename = "@AdditionalCost",
        default,
        deserialize_with = "de_opt_from_str"
    )]
    pub additional_cost: Option<f64>,
    /// The currency (for example `"USD"`).
    #[serde(rename = "@Currency")]
    pub currency: String,
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

    #[test]
    fn parses_get_pricing_response() {
        let body = r#"<ApiResponse Status="OK" xmlns="http://api.namecheap.com/xml.response">
          <Errors />
          <CommandResponse Type="namecheap.users.getPricing">
            <UserGetPricingResult>
              <ProductType Name="domains">
                <ProductCategory Name="register">
                  <Product Name="com">
                    <Price Duration="1" DurationType="YEAR" Price="13.98" RegularPrice="13.98" YourPrice="13.98" AdditionalCost="0.20" PromotionPrice="0.0" Currency="USD" />
                    <Price Duration="2" DurationType="YEAR" Price="26.26" RegularPrice="13.98" YourPrice="26.26" AdditionalCost="0.20" PromotionPrice="0.0" Currency="USD" />
                  </Product>
                </ProductCategory>
              </ProductType>
            </UserGetPricingResult>
          </CommandResponse>
        </ApiResponse>"#;

        let payload: GetPricingPayload = crate::response::parse(StatusCode::OK, body).unwrap();
        let result = PricingResult {
            product_types: payload.result.product_types,
        };
        assert_eq!(result.product_types.len(), 1);
        assert_eq!(result.product_types[0].name, "domains");
        assert_eq!(result.product_types[0].categories[0].name, "register");
        assert_eq!(
            result.product_types[0].categories[0].products[0].name,
            "com"
        );

        let prices = result.prices();
        assert_eq!(prices.len(), 2);
        assert_eq!(prices[0].duration, 1);
        assert_eq!(prices[0].price, 13.98);
        assert_eq!(prices[0].regular_price, 13.98);
        assert_eq!(prices[0].additional_cost, Some(0.20));
        assert_eq!(prices[0].currency, "USD");
        assert_eq!(prices[1].duration, 2);
        assert_eq!(prices[1].price, 26.26);
    }

    #[test]
    fn pricing_request_builds_params() {
        let params = PricingRequest::domains()
            .action("REGISTER")
            .tld("com")
            .to_params();
        let get = |key: &str| {
            params
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.as_str())
        };
        assert_eq!(get("ProductType"), Some("DOMAIN"));
        assert_eq!(get("ProductCategory"), Some("DOMAINS"));
        assert_eq!(get("ActionName"), Some("REGISTER"));
        assert_eq!(get("ProductName"), Some("com"));
    }
}
