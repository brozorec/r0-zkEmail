use anyhow::Result;
use base64::Engine;
use serde::{Deserialize, Serialize};
use stellar_xdr::curr::{
    Hash, HostFunction, InvokeContractArgs, Limits, ScAddress, ScBytes, ScSymbol,
    ScVal, SorobanAuthorizationEntry, WriteXdr,
};

#[derive(Serialize)]
struct RelayerRequest {
    params: RelayerParams,
}

#[derive(Serialize)]
struct RelayerParams {
    func: String,
    auth: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct RelayerResponse {
    pub success: bool,
    pub data: Option<TransactionData>,
    pub error: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug)]
pub struct TransactionData {
    pub hash: String,
    pub status: String,
    #[serde(rename = "transactionId")]
    pub transaction_id: String,
}

/// Build the InvokeContractArgs for verify_and_pay function
pub fn build_verify_and_pay_args(
    contract_address: &str,
    seal: Vec<u8>,
    image_id: Vec<u8>,
    journal: Vec<u8>,
) -> Result<InvokeContractArgs> {
    // Convert contract address to ScAddress
    let contract_addr = stellar_strkey::Contract::from_string(contract_address)?;
    let contract_address = ScAddress::Contract(Hash(contract_addr.0));

    // Build function arguments
    let args = vec![
        ScVal::Bytes(ScBytes(seal.try_into()?)),
        ScVal::Bytes(ScBytes(image_id.try_into()?)),
        ScVal::Bytes(ScBytes(journal.try_into()?)),
    ];

    Ok(InvokeContractArgs {
        contract_address,
        function_name: ScSymbol("verify_and_pay".try_into()?),
        args: args.try_into()?,
    })
}

/// Send a HostFunction to the OpenZeppelin relayer
pub async fn send_to_relayer(
    api_url: &str,
    api_key: &str,
    host_function: &HostFunction,
    auth_entries: Vec<SorobanAuthorizationEntry>,
) -> Result<RelayerResponse> {
    let client = reqwest::Client::new();

    // Encode HostFunction to base64 XDR
    let func_xdr_bytes = host_function.to_xdr(Limits::none())?;
    let func_xdr = base64::engine::general_purpose::STANDARD.encode(&func_xdr_bytes);

    // Encode auth entries to base64 XDR
    let auth_xdr: Result<Vec<String>> = auth_entries
        .iter()
        .map(|entry| {
            let xdr_bytes = entry.to_xdr(Limits::none())?;
            Ok(base64::engine::general_purpose::STANDARD.encode(&xdr_bytes))
        })
        .collect();

    // Build request payload
    let payload = RelayerRequest {
        params: RelayerParams {
            func: func_xdr,
            auth: auth_xdr?,
        },
    };

    tracing::info!("Sending transaction to relayer: {}", api_url);
    tracing::debug!("Request payload: {}", serde_json::to_string_pretty(&payload)?);

    // Make POST request
    let response = client
        .post(api_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?;

    let status = response.status();

    if !status.is_success() {
        let error_text = response.text().await?;
        tracing::error!("Relayer request failed with status {}: {}", status, error_text);
        anyhow::bail!("Relayer request failed: {}", error_text);
    }

    let relayer_response: RelayerResponse = response.json().await?;

    if relayer_response.success {
        if let Some(data) = &relayer_response.data {
            tracing::info!(
                "Transaction submitted successfully - Hash: {}, Status: {}, ID: {}",
                data.hash,
                data.status,
                data.transaction_id
            );
        }
    } else {
        if let Some(error) = &relayer_response.error {
            tracing::error!("Relayer reported error: {}", serde_json::to_string_pretty(error)?);
        }
        anyhow::bail!("Relayer reported failure");
    }

    Ok(relayer_response)
}
