# Hyperlane CLI Tool

A command-line interface for sending and querying Hyperlane messages across different blockchains.

## Features

- **Send Messages**: Dispatch messages via Hyperlane protocol to any supported destination chain
- **Search Messages**: Query and filter messages using event filtering
- **Multi-chain Support**: Works with any EVM-compatible chain that has Hyperlane deployed
- **Gas Fee Estimation**: Automatically quotes and pays required interchain gas fees

## Installation

From the root of the Hyperlane monorepo:

```bash
cd rust/main
cargo build --release --package hyperlane-cli-tool
```

The binary will be available at:
```bash
./target/release/hyperlane-cli
```

## ⚠️ Getting Started - Testnet First!

**Always test on testnets before using mainnet.** This prevents costly mistakes and allows you to familiarize yourself with the tool.

### Step 1: Test on Sepolia Testnet

1. **Get Sepolia ETH**: Use a faucet like [Sepolia Faucet](https://sepoliafaucet.com/) to get test ETH
2. **Test a simple message** to Base Sepolia:

```bash
./target/release/hyperlane-cli send \
  --rpc-url "https://sepolia.infura.io/v3/YOUR-API-KEY" \
  --mailbox-address "0xfFAEF09B3cd11D9b20d1a19bECca54EEC2884766" \
  --destination-chain 84532 \
  --destination-address "0x783c4a0bB6663359281aD4a637D5af68F83ae213" \
  --message "0x746872696c6c656420746f20636f6e74726962757465" \
  --private-key "0xYOUR-PRIVATE-KEY"
```

3. **Verify** the message was sent by checking [Hyperlane Explorer](https://explorer.hyperlane.xyz/)

### Step 2: Test Cross-chain on Multiple Testnets

Once Sepolia works, test between different testnets:
- Sepolia → Base Sepolia
- Sepolia → Arbitrum Sepolia  
- Sepolia → Optimism Sepolia

## Usage

### Sending Messages

Send a message from one chain to another:

```bash
./target/release/hyperlane-cli send \
  --rpc-url "https://eth-mainnet.g.alchemy.com/v2/YOUR_API_KEY" \
  --mailbox-address "0x2f2aFaE1139Ce54feFC27266523122DC4639C06F" \
  --destination-chain 42161 \
  --destination-address "0x0000000000000000000000000000000000000000" \
  --message "0x746872696c6c656420746f20636f6e74726962757465" \
  --private-key "0xYOUR-PRIVATE-KEY"
```

#### Parameters:
- `--rpc-url`: RPC endpoint for the origin chain
- `--mailbox-address`: Hyperlane Mailbox contract address on origin chain
- `--destination-chain`: Destination chain domain ID (not chain ID)
- `--destination-address`: Recipient address on destination chain (20 or 32 bytes)
- `--message`: Message data as hex string (with or without 0x prefix)
- `--private-key`: Private key for signing the transaction

> ⚠️ **Important**: The destination address must be a contract that implements the `IMessageRecipient` interface with a `handle(uint32, bytes32, bytes)` function. For testing, you can use the zero address `0x0000000000000000000000000000000000000000` which will allow message delivery without executing any handle function, or deploy your own test recipient contract using the example below.

### Searching Messages

The CLI provides two approaches for searching messages sent from a specific chain: basic filtering and advanced MatchingList filtering.

#### Basic Message Search

For simple queries, you can use basic filters to search for messages:

```bash
./target/release/hyperlane-cli search \
  --rpc-url "https://eth-mainnet.g.alchemy.com/v2/YOUR_API_KEY" \
  --mailbox-address "0xc005dc82818d67AF737725bD4bf75435d065D239" \
  --origin 1 \
  --destination 10 \
  --from-block 23449987 \
  --to-block 23449996
```

#### Advanced Search with MatchingList

For complex filtering scenarios, the CLI supports Hyperlane's MatchingList system through JSON configuration. The MatchingList allows you to specify detailed criteria for message matching using a combination of origin domain, sender address, destination domain, and recipient address filters. Each filter can be configured as a wildcard (matches all), a single value (exact match), or multiple values (matches any).

```bash
./target/release/hyperlane-cli search \
  --rpc-url "https://eth-mainnet.g.alchemy.com/v2/N-Gnpjy1WvCfokwj6fiOfuAVL_At6IvE" \
  --mailbox-address "0xc005dc82818d67AF737725bD4bf75435d065D239" \
  --from-block 23449987 \
  --to-block 23449996 \
  --matching-list '[{"originDomain": "*", "senderAddress": "*", "destinationDomain": [42161, 10], "recipientAddress": "*"}]'
```

The MatchingList JSON format accepts an array of rules where each rule contains four optional fields. When a field is omitted, it defaults to wildcard matching. For single values, you can specify the value directly. For multiple values, use an array notation. The wildcard "*" explicitly matches all values for that field. All conditions within a rule must be satisfied for a message to match, while having multiple rules creates an OR relationship between them.

#### Search Parameters:
- `--rpc-url`: RPC endpoint for the chain to search
- `--mailbox-address`: Hyperlane Mailbox contract address
- `--origin`: Filter by origin domain (optional, basic filtering only)
- `--destination`: Filter by destination domain (optional, basic filtering only) 
- `--from-block`: Start block number (optional, defaults to 0)
- `--to-block`: End block number (optional, defaults to latest)
- `--matching-list`: JSON array of MatchingList rules for advanced filtering (optional)



## Configuration

### Official Hyperlane Deployments

#### Testnet Addresses 

| Chain | Domain ID | Mailbox Address | RPC URL Example |
|-------|-----------|-----------------|-----------------|
| Sepolia | 11155111 | `0xfFAEF09B3cd11D9b20d1a19bECca54EEC2884766` | `https://sepolia.infura.io/v3/YOUR-KEY` |
| Base Sepolia | 84532 | `0x6966b0E55883d49BFB24539356a2f8A673E02039` | `https://sepolia.base.org` |
| Arbitrum Sepolia | 421614 | `0x598facE78a4302f11E3de0bee1894Da0b2Cb71F8` | `https://sepolia-rollup.arbitrum.io/rpc` |
| Optimism Sepolia | 11155420 | `0x6966b0E55883d49BFB24539356a2f8A673E02039` | `https://sepolia.optimism.io` |

#### Mainnet Addresses 

| Chain | Domain ID | Mailbox Address | RPC URL Example |
|-------|-----------|-----------------|-----------------|
| Ethereum | 1 | `0xc005dc82818d67AF737725bD4bf75435d065D239` | `https://mainnet.infura.io/v3/YOUR-KEY` |
| Arbitrum | 42161 | `0x979Ca5202784112f4738403dBec5D0F3B9daabB9` | `https://arb1.arbitrum.io/rpc` |
| Optimism | 10 | `0xd4C1905BB1D26BC93DAC913e13CaCC278CdCC80D` | `https://mainnet.optimism.io` |
| Base | 8453 | `0xeA87ae93Fa0019a82A727bfd3eBd1cFCa8f64f1D` | `https://mainnet.base.org` |
| Polygon | 137 | `0x5d934f4e2f797775e53561bB72aca21ba36B96BB` | `https://polygon-rpc.com` |

> ⚠️ **Important**: Domain IDs are different from chain IDs! Always use the Domain ID from the table above.

## Testing Guide

### Prerequisites

1. **Get Test ETH**: 
   - Sepolia: [Sepolia Faucet](https://sepoliafaucet.com/)
   - Base Sepolia: [Base Faucet](https://bridge.base.org/deposit)
   - Arbitrum Sepolia: [Arbitrum Faucet](https://faucet.quicknode.com/arbitrum/sepolia)

2. **RPC Access**: Get API keys from:
   - [Infura](https://infura.io/) for Ethereum chains
   - [Alchemy](https://www.alchemy.com/) as alternative
   - Use public RPCs for testing (rate limited)


## Architecture

### Components

1. **Send Module** (`send.rs`):
   - Connects to origin chain via ethers-rs
   - Estimates gas fees using `quoteDispatch`
   - Sends message via Mailbox `dispatch` function
   - Returns transaction hash and message ID

2. **Search Module** (`search.rs`):
   - Queries Dispatch events from Mailbox contract
   - Applies MatchingList filters for advanced message matching
   - Displays message details and transaction information

3. **MatchingList Integration**:
   - Uses Hyperlane's MatchingList system for flexible message filtering
   - Supports filtering by origin, destination, sender, recipient, and message content



