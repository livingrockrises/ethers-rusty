use ethers::prelude::*;
use std::sync::Arc;
use std::env;
use dotenv::dotenv;

abigen!(
    MyContract,
    "./abi.json" // save your ABI to a file called `abi.json` in the project root
);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let rpc_url = env::var("RPC_URL")?;
    let private_key = env::var("PRIVATE_KEY")?;
    let chain_id: u64 = env::var("CHAIN_ID")?.parse()?;
    let contract_address: Address = env::var("CONTRACT_ADDRESS")?.parse()?;

    let user: Address = env::var("USER_ADDRESS")?.parse()?;
    let token: Address = env::var("TOKEN_ADDRESS")?.parse()?;
    let amount: U256 = U256::from_dec_str(&env::var("AMOUNT")?)?;
    let nonce: U256 = env::var("NONCE")?.parse()?;
    let signature: Bytes = env::var("SIGNATURE")?.parse()?; // or hex::decode + Bytes::from

    println!("=== Configuration ===");
    println!("RPC URL: {}", rpc_url);
    println!("Chain ID: {}", chain_id);
    println!("Contract Address: {:?}", contract_address);
    println!("User Address: {:?}", user);
    println!("Token Address: {:?}", token);
    println!("Amount: {}", amount);
    println!("Nonce: {}", nonce);
    println!("Signature: 0x{}", hex::encode(&signature));
    println!();

    let provider = Provider::<Http>::try_from(rpc_url)?;
    let wallet = private_key.parse::<LocalWallet>()?.with_chain_id(chain_id);
    let wallet_address = wallet.address();
    let client = SignerMiddleware::new(provider, wallet);
    let client = Arc::new(client);

    println!("=== Wallet Information ===");
    println!("Wallet Address: {:?}", wallet_address);
    
    // Check wallet balance
    let balance = client.get_balance(wallet_address, None).await?;
    println!("Wallet Balance: {} ETH", ethers::utils::format_units(balance, "ether")?);
    
    // Check user balance
    let user_balance = client.get_balance(user, None).await?;
    println!("User Balance: {} ETH", ethers::utils::format_units(user_balance, "ether")?);
    println!();

    // Check if balance is sufficient for the transaction
    let gas_price = client.get_gas_price().await?;
    println!("Current Gas Price: {} Gwei", ethers::utils::format_units(gas_price, "gwei")?);
    
    // Estimate gas for the transaction
    let contract = MyContract::new(contract_address, client.clone());
    let call = contract.lock(user, token, amount, nonce, signature).value(amount);
    
    println!("=== Transaction Details ===");
    println!("Transaction Value: {} ETH", ethers::utils::format_units(amount, "ether")?);
    
    // Try to estimate gas (this might fail if there are insufficient funds)
    match call.estimate_gas().await {
        Ok(gas_estimate) => {
            println!("Estimated Gas: {}", gas_estimate);
            let total_cost = gas_estimate * gas_price + amount;
            println!("Total Transaction Cost: {} ETH", ethers::utils::format_units(total_cost, "ether")?);
            
            if total_cost > balance {
                println!("❌ INSUFFICIENT FUNDS: Need {} ETH, but wallet has {} ETH", 
                    ethers::utils::format_units(total_cost, "ether")?,
                    ethers::utils::format_units(balance, "ether")?);
                return Ok(());
            } else {
                println!("✅ Sufficient funds available");
            }
        }
        Err(e) => {
            println!("❌ Failed to estimate gas: {:?}", e);
            println!("This might be due to insufficient funds or invalid parameters");
        }
    }
    println!();

    println!("=== Sending Transaction ===");
    let tx = call.send().await?;

    println!("Transaction Hash: {:?}", tx.tx_hash());
    println!("Waiting for transaction to be mined...");
    
    let receipt = tx.await?;
    match receipt {
        Some(r) => {
            println!("✅ Transaction mined in block: {:?}", r.block_number);
            println!("Gas Used: {}", r.gas_used.unwrap_or_default());
            println!("Status: {}", if r.status.unwrap_or_default() == U64::from(1) { "Success" } else { "Failed" });
        }
        None => {
            println!("❌ Transaction receipt not found");
        }
    }

    Ok(())
}
