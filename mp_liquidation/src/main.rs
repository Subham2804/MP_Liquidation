mod constants;
use anyhow::Error;
use async_trait::async_trait;
use base64;
use cosmwasm_std::{Coin, Decimal, Uint128};
use reqwest;
use serde_json;
use tokio::time::{interval, Duration};

async fn fetch_financial_data(user_address: String, query_type: &str) -> Result<Vec<Coin>, Error> {
    let query = format!(r#"{{"{}": {{"user": "{}"}}}}"#, query_type, user_address);
    let query_msg_base64 = base64::encode(&query);
    let url = format!(
        "{}/cosmwasm/wasm/v1/contract/{}/smart/{}",
        constants::REST_ENDPOINT,
        constants::REDBANK_CONTRACT,
        query_msg_base64
    );

    let response = reqwest::get(&url).await.map_err(Error::new)?;
    if response.status().is_success() {
        let body = response
            .json::<serde_json::Value>()
            .await
            .map_err(Error::new)?;
        let mut financial_data: Vec<Coin> = vec![];
        if let Some(data) = body["data"].as_array() {
            for item in data {
                let amount = item["amount"]
                    .as_str()
                    .unwrap_or_default()
                    .parse::<u128>()
                    .unwrap_or_default();
                let denom = item["denom"].as_str().unwrap_or_default();
                financial_data.push(Coin {
                    amount: Uint128::from(amount),
                    denom: denom.to_string(),
                });
            }
        }
        Ok(financial_data)
    } else {
        Err(anyhow::anyhow!(
            "Request failed with status: {}",
            response.status()
        ))
    }
}

async fn get_user_financials(user_address: String) -> Result<(Vec<Coin>, Vec<Coin>), Error> {
    let debts = fetch_financial_data(user_address.clone(), "user_debts").await?;
    let collaterals = fetch_financial_data(user_address, "user_collaterals").await?;
    Ok((debts, collaterals))
}

fn get_price(denom: &str) -> Result<u128, Error> {
    //// Setting a default price of 1 USD for now (6 decimal places)
    return Ok(1000000);
}

fn get_token_value(collateral: Coin, token_decimals: u128) -> Result<u128, Error> {
    let price = get_price(&collateral.denom)?;
    let collateral_amount = collateral.amount.u128();
    let scaled_collateral = collateral_amount * price;
    let normalized_collateral = if token_decimals > 0 {
        scaled_collateral.saturating_div(10^token_decimals)
    } else {
        scaled_collateral // No division needed if token_decimals is 0, though unlikely.
    };

    let collateral_value = normalized_collateral.saturating_mul(price);
    Ok(collateral_value)
}

#[tokio::main]
async fn main() {

    // For example, to run every 60 seconds, set it to 60.
    const DURATION: u64 = 10;

    let mut interval = interval(Duration::from_secs(DURATION));

    loop {
        interval.tick().await;


    // Define the user address as a constant string to avoid repetition.
    let user_address = "osmo1p2lnskywgtmdszw4lyka8wu3mn925365djuc24".to_string();

    // Perform a single asynchronous call to fetch the user's financials (debts and collaterals).
    match get_user_financials(user_address).await {
        Ok((debts, collaterals)) => {
            // TASK 1: Output the user's debts and collaterals.
            println!("Debts: {:?}", debts);
            println!("Collaterals: {:?}", collaterals);

            // TASK 2: Calculate and output the collateralization ratio.
            // Since we already have the debts and collaterals, we can directly pass them to
            // the function without making another call to `get_user_financials`.
            match calculate_collateralization_ratio(debts, collaterals) {
                Ok(collateralization_ratio) => {
                    println!("Collateralization ratio: {}", collateralization_ratio);
                },
                Err(e) => println!("Failed to calculate the collateralization ratio: {}", e),
            }
        },
        Err(e) => println!("Request failed: {}", e),
    }

    }
}

pub fn calculate_collateralization_ratio(
    user_debt: Vec<Coin>,
    user_collateral: Vec<Coin>,
) -> Result<Decimal, Error> {

    //// NOTE : token_decimals initialized to 6 for now, as we are assuming the token to be of 6 decimal places , any token decimal would be fetched contract token info section or elsewhere
    let mut total_collateral_value: u128 = 0;
    let mut total_debt_value: u128 = 0;
    for collateral in user_collateral {
        let token_decimals = 6;
        let collateral_value = get_token_value(collateral, token_decimals)?;
        total_collateral_value += collateral_value;
    }
    for debt in user_debt {
        let token_decimals = 6;
        let debt_value = get_token_value(debt, token_decimals)?;
        total_debt_value += debt_value;
    }
    let collateralization_ratio = if total_debt_value > 0 {
        Decimal::from_ratio(total_collateral_value, total_debt_value)
    } else {
        Decimal::zero()
    };
    Ok(collateralization_ratio)
}