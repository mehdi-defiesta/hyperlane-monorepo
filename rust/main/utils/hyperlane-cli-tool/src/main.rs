use clap::{Parser, Subcommand};
use anyhow::{Result, Context};
use ethers::prelude::*;
use hyperlane_core::H256;
use std::str::FromStr;

mod send;
mod search;
mod config;
mod matching_list;

use send::{send_message, check_delivery};
use search::search_messages;
use config::Config;


#[derive(Parser)]
#[command(name = "hyperlane-cli")]
#[command(about = "A CLI tool for sending and querying Hyperlane messages")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Send a message via Hyperlane
    Send {
        /// Origin chain RPC URL
        #[arg(long)]
        rpc_url: String,
        
        /// Mailbox contract address on origin chain
        #[arg(long)]
        mailbox_address: String,
        
        /// Destination chain ID
        #[arg(long)]
        destination_chain: u32,
        
        /// Destination address (32 bytes hex)
        #[arg(long)]
        destination_address: String,
        
        /// Message bytes (hex encoded)
        #[arg(long)]
        message: String,
        
        /// Private key for signing transactions
        #[arg(long)]
        private_key: String,
    },
    /// Search for messages sent from a chain
    Search {
        /// Chain RPC URL to search
        #[arg(long)]
        rpc_url: String,
        
        /// Mailbox contract address
        #[arg(long)]
        mailbox_address: String,
        
        /// Origin filter (optional)
        #[arg(long)]
        origin: Option<u32>,
        
        /// Destination filter (optional)
        #[arg(long)]
        destination: Option<u32>,
        
        /// Start block number (optional)
        #[arg(long)]
        from_block: Option<u64>,
        
        /// End block number (optional)
        #[arg(long)]
        to_block: Option<u64>,
        
        /// MatchingList JSON for advanced filtering (optional)
        #[arg(long)]
        matching_list: Option<String>,
    },
    /// Check if a message was delivered
    CheckDelivery {
        /// Destination chain RPC URL
        #[arg(long)]
        rpc_url: String,
        
        /// Mailbox contract address on destination chain
        #[arg(long)]
        mailbox_address: String,
        
        /// Message ID to check
        #[arg(long)]
        message_id: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing with info level
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Send {
            rpc_url,
            mailbox_address,
            destination_chain,
            destination_address,
            message,
            private_key,
        } => {
            let config = Config {
                rpc_url,
                mailbox_address: mailbox_address.parse()
                    .context("Invalid mailbox address")?,
                private_key,
            };
            
            let destination_addr = parse_address(&destination_address)?;
            let message_bytes = hex::decode(message.strip_prefix("0x").unwrap_or(&message))
                .context("Invalid message hex")?;
            
            send_message(
                config,
                destination_chain,
                destination_addr,
                message_bytes,
            ).await?;
        }
        Commands::Search {
            rpc_url,
            mailbox_address,
            origin,
            destination,
            from_block,
            to_block,
            matching_list,
        } => {
            let config = Config {
                rpc_url,
                mailbox_address: mailbox_address.parse()
                    .context("Invalid mailbox address")?,
                private_key: String::new(), // Not needed for search
            };
            
            search_messages(
                config,
                origin,
                destination,
                from_block,
                to_block,
                matching_list,
            ).await?;
        }
        Commands::CheckDelivery {
            rpc_url,
            mailbox_address,
            message_id,
        } => {
            check_delivery(
                rpc_url,
                mailbox_address,
                message_id,
            ).await?;
        }
    }
    
    Ok(())
}

fn parse_address(addr: &str) -> Result<H256> {
    let addr = addr.strip_prefix("0x").unwrap_or(addr);
    
    if addr.len() == 40 {
        // 20-byte address, pad to 32 bytes
        let address: Address = addr.parse().context("Invalid 20-byte address")?;
        let mut bytes = [0u8; 32];
        bytes[12..32].copy_from_slice(address.as_bytes());
        Ok(H256::from(bytes))
    } else if addr.len() == 64 {
        // 32-byte address
        H256::from_str(addr).context("Invalid 32-byte address")
    } else {
        anyhow::bail!("Address must be either 20 bytes (40 hex chars) or 32 bytes (64 hex chars)")
    }
}