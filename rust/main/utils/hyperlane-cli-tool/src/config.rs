use ethers::types::Address;

#[derive(Debug, Clone)]
pub struct Config {
    pub rpc_url: String,
    pub mailbox_address: Address,
    pub private_key: String,
}