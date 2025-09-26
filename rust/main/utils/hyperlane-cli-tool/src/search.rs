use anyhow::{Result, Context};
use ethers::prelude::*;
use ethers::providers::{Http, Provider};
use hyperlane_core::H256 as HyperlaneH256;

use crate::config::Config;
use crate::matching_list::MatchingList;

// Mailbox ABI for event querying
abigen!(
    MailboxEvents,
    r#"[
        event Dispatch(address indexed sender, uint32 indexed destination, bytes32 indexed recipient, bytes message)
        event DispatchId(bytes32 indexed messageId)
    ]"#
);

pub async fn search_messages(
    config: Config,
    origin_filter: Option<u32>,
    destination_filter: Option<u32>,
    from_block: Option<u64>,
    to_block: Option<u64>,
    matching_list_json: Option<String>,
) -> Result<()> {
    println!("🔍 Hyperlane Message Search");
    println!("═══════════════════════════════════════════════════════════════════════════════");
    
    println!("RPC URL:              {}", config.rpc_url);
    println!("Mailbox:              {:?}", config.mailbox_address);
    
    if let Some(origin) = origin_filter {
        println!("Origin Filter:        {}", origin);
    } else {
        println!("Origin Filter:        All origins");
    }
    
    // Display destination filter - show actual destinations from MatchingList if available
    if let Some(json) = &matching_list_json {
        // Parse MatchingList early to show destination filters
        match MatchingList::from_json(json) {
            Ok(list) => {
                let destinations: Vec<String> = list.rules.iter()
                    .map(|rule| format!("{}", rule.destination_domain))
                    .collect::<std::collections::HashSet<_>>()
                    .into_iter()
                    .collect();
                
                if destinations.len() == 1 && destinations[0] == "*" {
                    println!("Destination Filter:   All destinations");
                } else {
                    println!("Destination Filter:   {}", destinations.join(", "));
                }
            }
            Err(_) => {
                if let Some(destination) = destination_filter {
                    println!("Destination Filter:   {}", destination);
                } else {
                    println!("Destination Filter:   All destinations");
                }
            }
        }
    } else if let Some(destination) = destination_filter {
        println!("Destination Filter:   {}", destination);
    } else {
        println!("Destination Filter:   All destinations");
    }
    
    if let Some(from) = from_block {
        println!("From Block:           {}", from);
    } else {
        println!("From Block:           Genesis (0)");
    }
    
    if let Some(to) = to_block {
        println!("To Block:             {}", to);
    } else {
        println!("To Block:             Latest");
    }
    
    // Parse MatchingList if provided
    let matching_list = if let Some(json) = &matching_list_json {
        match MatchingList::from_json(json) {
            Ok(list) => {
                println!("MatchingList:         {} rules", list.rules.len());
                Some(list)
            }
            Err(e) => {
                println!("MatchingList:         Invalid JSON - {}", e);
                return Err(e);
            }
        }
    } else {
        println!("MatchingList:         None (all messages)");
        None
    };
    
    println!("═══════════════════════════════════════════════════════════════════════════════\n");
    
    // Check for unsupported filtering
    if origin_filter.is_some() && matching_list.is_none() {
        println!("⚠️  Warning: Origin filtering is not supported in basic mode.");
        println!("    Dispatch events don't contain origin domain information.");
        println!("    Use --matching-list for origin-based filtering.\n");
    }
    
    // Setup provider
    let provider = Provider::<Http>::try_from(&config.rpc_url)
        .context("Failed to create provider")?;
    
    // Create mailbox contract instance
    let mailbox = MailboxEvents::new(config.mailbox_address, std::sync::Arc::new(provider.clone()));
    
    // Determine block range
    let latest_block = provider.get_block_number().await
        .context("Failed to get latest block number")?;
    
    let from_block = from_block.unwrap_or(0);
    let to_block = to_block.unwrap_or(latest_block.as_u64());
    
    println!("🔎 Searching blocks {} to {}", from_block, to_block);
    
    // Create filter for Dispatch events
    let mut filter = mailbox
        .dispatch_filter()
        .from_block(from_block)
        .to_block(to_block);
    
    // Apply destination filter if provided (destination is topic2 in the Dispatch event)
    if let Some(dest) = destination_filter {
        filter = filter.topic2(U256::from(dest));
    }
    
    println!("📡 Querying events...");
    let events = filter.query().await
        .context("Failed to query dispatch events")?;
    
    println!("📊 Found {} dispatch events\n", events.len());
    
    let mut matched_count = 0;
    
    if events.is_empty() {
        println!("📭 No messages found in the specified range.");
        println!("💡 Try expanding the block range or removing filters.\n");
        return Ok(());
    }
    
    println!("📋 Message Details:");
    println!("───────────────────────────────────────────────────────────────────────────────");
    
    for (i, dispatch_event) in events.iter().enumerate() {
        // Apply MatchingList filtering if provided, otherwise include all events
        let matches = if let Some(ref matching_list) = matching_list {
            // Use MatchingList for advanced filtering
            matching_list.matches_message(
                origin_filter.unwrap_or(0), // We don't have origin in event, use filter or 0
                &dispatch_event.sender,
                dispatch_event.destination,
                &HyperlaneH256::from(dispatch_event.recipient),
            )
        } else {
            // For basic filtering, ethers query already applied destination filter
            // Note: origin filtering not supported in basic mode (no origin info in Dispatch events)
            true
        };
        
        if matches {
            matched_count += 1;
            
            println!("Message #{}:", matched_count);
            println!("  Sender:       {:?}", dispatch_event.sender);
            println!("  Destination:  {}", dispatch_event.destination);
            println!("  Recipient:    0x{}", hex::encode(dispatch_event.recipient));
            println!("  Message Size: {} bytes", dispatch_event.message.len());
            
            // Show message data preview  
            let msg_preview = if dispatch_event.message.len() > 32 {
                format!("0x{}...", &hex::encode(&dispatch_event.message)[..64])
            } else {
                format!("0x{}", hex::encode(&dispatch_event.message))
            };
            println!("  Message Data: {}", msg_preview);
            println!("───────────────────────────────────────────────────────────────────────────────");
        }
    }
    
    if matched_count == 0 {
        println!("No matching messages found");
        println!("───────────────────────────────────────────────────────────────────────────────");
    }
    
    println!("📊 Search Summary:");
    println!("───────────────────────────────────────────────────────────────────────────────");
    println!("Total Events:         {}", events.len());
    println!("Matching Messages:    {}", matched_count);
    println!("Blocks Searched:      {}", to_block - from_block + 1);
    println!("───────────────────────────────────────────────────────────────────────────────");
    
    Ok(())
}