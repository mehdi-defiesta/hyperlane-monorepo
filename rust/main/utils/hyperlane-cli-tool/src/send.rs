use anyhow::{Result, Context};
use ethers::prelude::*;
use ethers::providers::{Http, Provider};
use ethers::signers::{LocalWallet, Signer};
use hyperlane_core::H256;

use crate::config::Config;


// Mailbox ABI - simplified for dispatch function
abigen!(
    Mailbox,
    r#"[
        function dispatch(uint32 destinationDomain, bytes32 recipientAddress, bytes calldata messageBody) external payable returns (bytes32 messageId)
        function quoteDispatch(uint32 destinationDomain, bytes32 recipientAddress, bytes calldata messageBody) external view returns (uint256 fee)
    ]"#
);

pub async fn send_message(
    config: Config,
    destination_chain: u32,
    destination_address: H256,
    message_bytes: Vec<u8>,
) -> Result<()> {
    println!("\n🚀 Hyperlane Message Dispatch");
    println!("═══════════════════════════════════════════════════════════════════════════════");
    
    println!("RPC URL:              {}", config.rpc_url);
    println!("Mailbox:              {:?}", config.mailbox_address);
    println!("Destination Chain:    {}", destination_chain);
    println!("Destination Address:  {:?}", destination_address);
    
    let msg_hex = hex::encode(&message_bytes);
    let msg_display = format!("0x{}", msg_hex);
    println!("Message:              {}", msg_display);
    println!("Message Length:       {} bytes", message_bytes.len());
    println!("═══════════════════════════════════════════════════════════════════════════════\n");
    
    // Setup provider and wallet
    let provider = Provider::<Http>::try_from(&config.rpc_url)
        .context("Failed to create provider")?;
    
    // Strip 0x prefix if present
    let private_key = config.private_key.strip_prefix("0x").unwrap_or(&config.private_key);
    let wallet: LocalWallet = private_key.parse()
        .context("Invalid private key")?;
    
    let chain_id = provider.get_chainid().await
        .context("Failed to get chain ID")?;
    
    let wallet = wallet.with_chain_id(chain_id.as_u64());
    let wallet_address = wallet.address();
    let client = std::sync::Arc::new(SignerMiddleware::new(provider, wallet));
    
    // Create mailbox contract instance
    let mailbox = Mailbox::new(config.mailbox_address, client.clone());
    
    // Convert destination address to bytes32
    let recipient_bytes32: [u8; 32] = destination_address.into();
    
    // Check wallet balance and estimate costs
    println!("💰 Preparing Transaction...");
    let balance = client.get_balance(wallet_address, None).await
        .context("Failed to get wallet balance")?;
    
    let fee = mailbox
        .quote_dispatch(destination_chain, recipient_bytes32, message_bytes.clone().into())
        .call()
        .await
        .context("Failed to quote dispatch fee")?;
    
    let gas_estimate = mailbox
        .dispatch(destination_chain, recipient_bytes32, message_bytes.clone().into())
        .value(fee)
        .estimate_gas()
        .await
        .context("Failed to estimate gas")?;
    
    // Display wallet and cost information
    println!("💰 Financial Summary");
    println!("───────────────────────────────────────────────────────────────────────────────");
    println!("Wallet Balance:       {} ETH ({} wei)", ethers::utils::format_ether(balance), balance);
    println!("Required Fee:         {} ETH ({} wei)", ethers::utils::format_ether(fee), fee);
    println!("Estimated Gas:        {} units", gas_estimate);
    println!("Status:               {}", if balance >= fee { "✅ Sufficient funds" } else { "❌ Insufficient funds" });
    println!("───────────────────────────────────────────────────────────────────────────────\n");
    
    // Check if we have enough balance
    if balance < fee {
        anyhow::bail!("❌ Insufficient balance! Need {} wei, but only have {} wei", fee, balance);
    }
    
    // Send the message with some extra gas buffer
    println!("📤 Sending Transaction...");
    
    let tx = mailbox
        .dispatch(destination_chain, recipient_bytes32, message_bytes.clone().into())
        .value(fee)
        .gas(gas_estimate * 120 / 100); // Add 20% buffer
    
    let pending_tx = tx.send().await
        .context("Failed to send dispatch transaction")?;
    
    println!("⏳ Transaction submitted: {}", pending_tx.tx_hash());
    println!("⏳ Waiting for confirmation...\n");
    
    let receipt = pending_tx.await
        .context("Transaction failed")?
        .ok_or_else(|| anyhow::anyhow!("No receipt returned"))?;
    
    // Calculate the message ID properly
    let message_id = mailbox
        .dispatch(destination_chain, recipient_bytes32, message_bytes.into())
        .value(fee)
        .call()
        .await
        .context("Failed to get message ID")?;
    
    // Display transaction results
    println!("✅ Transaction Results");
    println!("═══════════════════════════════════════════════════════════════════════════════");
    println!("Status:               ✅ Success");
    println!("Transaction Hash:     {:?}", receipt.transaction_hash);
    println!("Gas Used:             {} units", receipt.gas_used.unwrap_or_default());
    println!("Block Number:         {}", receipt.block_number.unwrap_or_default());
    
    println!("Message ID:           0x{}", hex::encode(message_id));
    println!("═══════════════════════════════════════════════════════════════════════════════\n");
    
    // Verification instructions
    println!("🔍 Verify Message Delivery:");
    println!("───────────────────────────────────────────────────────────────────────────────");
    println!("\x1b[32mMethod 1: Hyperlane Explorer\x1b[0m");
    println!("  URL: https://explorer.hyperlane.xyz/");
    println!("  Search: {:?}", receipt.transaction_hash);
    println!();
    println!("\x1b[32mMethod 2: CLI Check\x1b[0m");
    
    // Determine destination RPC URL and mailbox address based on destination chain
    let (dest_rpc_url, dest_mailbox_address) = match destination_chain {
        84532 => ("https://sepolia.base.org", "0x6966b0E55883d49BFB24539356a2f8A673E02039"), // Base Sepolia
        11155420 => ("https://sepolia.optimism.io", "0x6966b0E55883d49BFB24539356a2f8A673E02039"), // Optimism Sepolia
        421614 => ("https://sepolia-rollup.arbitrum.io/rpc", "0x598facE78a4302f11E3de0bee1894Da0b2Cb71F8"), // Arbitrum Sepolia
        _ => ("https://sepolia.base.org", "0x6966b0E55883d49BFB24539356a2f8A673E02039"), // Default to Base Sepolia
    };
    
    println!("  ./target/release/hyperlane-cli check-delivery \\");
    println!("    --rpc-url \"{}\" \\", dest_rpc_url);
    println!("    --mailbox-address \"{}\" \\", dest_mailbox_address);
    println!("    --message-id \"0x{}\"", hex::encode(message_id));
    println!("───────────────────────────────────────────────────────────────────────────────");
    
    Ok(())
}

pub async fn check_delivery(
    rpc_url: String,
    mailbox_address: String,
    message_id: String,
) -> Result<()> {
    use ethers::prelude::*;
    use ethers::providers::{Http, Provider};
    
    println!("🔍 Checking message delivery...");
    println!("📍 RPC URL: {}", rpc_url);
    println!("📮 Mailbox: {}", mailbox_address);
    println!("🆔 Message ID: {}", message_id);
    
    // Setup provider
    let provider = Provider::<Http>::try_from(&rpc_url)
        .context("Failed to create provider")?;
    
    // Parse addresses
    let mailbox_addr: Address = mailbox_address.parse()
        .context("Invalid mailbox address")?;
    let msg_id: H256 = message_id.parse()
        .context("Invalid message ID")?;
    
    // Enhanced ABI for delivery checking and events
    abigen!(
        DeliveryChecker,
        r#"[
            function delivered(bytes32 messageId) external view returns (bool)
            event ProcessId(bytes32 indexed messageId)
        ]"#
    );
    
    let provider_arc = std::sync::Arc::new(provider);
    let checker = DeliveryChecker::new(mailbox_addr, provider_arc.clone());
    
    println!("───────────────────────────────────────────────────────────────────────────────");
    
    // Test the connection first
    println!("🔗 Testing connection to RPC...");
    match provider_arc.get_block_number().await {
        Ok(block_number) => {
            println!("✅ Connected to chain, latest block: {}", block_number);
        }
        Err(e) => {
            println!("❌ Failed to connect to RPC: {}", e);
            return Ok(());
        }
    }
    
    // Check if the mailbox contract exists
    println!("📋 Checking mailbox contract...");
    match provider_arc.get_code(mailbox_addr, None).await {
        Ok(code) => {
            if code.is_empty() {
                println!("❌ No contract found at mailbox address {}", mailbox_addr);
                return Ok(());
            } else {
                println!("✅ Mailbox contract found (code length: {} bytes)", code.len());
            }
        }
        Err(e) => {
            println!("❌ Failed to check mailbox contract: {}", e);
            return Ok(());
        }
    }
    
    println!("🔍 Querying message delivery status...");
    match checker.delivered(msg_id.into()).call().await {
        Ok(delivered) => {
            if delivered {
                println!("✅ Message has been delivered!");
            } else {
                println!("⏳ Message not yet delivered (still processing or failed)");
                println!();
                println!("🔍 Debugging tips:");
                println!("   - Verify the message ID is correct (should be 64 hex characters)");
                println!("   - Check if enough time has passed (delivery can take 1-5 minutes)");
                println!("   - Ensure you're checking the correct destination chain");
                println!("   - Message ID should be from the 'Message ID' field in transaction logs");
                println!();
                println!("💡 Check Hyperlane Explorer for details: https://explorer.hyperlane.xyz/");
            }
        }
        Err(e) => {
            println!("❌ Failed to check delivery status: {}", e);
            println!();
            println!("💡 This might be because:");
            println!("   - The delivered() function may not exist on this contract");
            println!("   - Wrong RPC URL or mailbox address");
            println!("   - Message ID format is incorrect (should be 0x followed by 64 hex chars)");
            println!("   - Network connectivity issues");
            println!("   - You might be checking the wrong destination chain");
        }
    }
    
    println!("───────────────────────────────────────────────────────────────────────────────");
    
    Ok(())
}