# Abstract

The bridge smart contract is used to connect Elrond to other blockchains. This is done by using a concept known as "Wrapped Tokens". A _wrapped eGLD_ for instance, is an ERC20 or similar token owned by an account on another blockchain, which is equivalent to a _real_ eGLD on the Elrond blockchain. Likewise, you can also transfer your tokens from the connected blockchains to hold them into your Elrond account.    

Users may also wrap/unwrap their eGLD at will. To wrap, simply send an amount of eGLD to the designated token manager contract (will be discussed later) and receive an equal amount of wrapped eGLD, in the form of ESDT. Unwrapping is the inverse process: send wrapped eGLD and you'll receive eGLD back.  

To be able to perform cross-chain transactions, we need an intermediary entity which will handle the translation between tokens from one chain to the other. This makes it easier to connect to multiple chains without having to perform the linking operation multiple times and have different standards for each chain. In this case, we're going to have PolyNetwork as our intermediary. You can read more about their standards [here](https://github.com/polynetwork/docs/tree/master/poly).  

So we're going to need at least two contracts: BlockHeaderSync and CrossChainManagement. We're also using a third contract called EsdtTokeManager. Next we're going to discuss each of those contracts and their role.  

# BlockHeaderSync Smart Contract

This contract is responsible for synchronizing the block headers, which will then be used to check the validity of transactions. A header has the following fields:  

```
struct Header {
    version: u32,
    chain_id: u64,
    prev_block_hash: H256,
    transactions_root: H256,
    cross_state_root: H256,
    block_root: H256,
    timestamp: u32,
    height: u32,
    consensus_data: u64,
	consensus_payload: Option<VbftBlockInfo>, 
	next_book_keeper: EthAddress,

	book_keepers: Vec<PublicKey>,
	sig_data: Vec<Signature>,
	block_hash: H256
}
```

`version` is the current header version.  
`chain_id` is the id of the linked chain. Headers are stored separately for each chain.  
`prev_block_hash` is the hash of the previous block.  
`transactions_root`, `cross_state_root` and `block_root` will probably be removed (merkle-proof related).  
`timestamp` is the block timestamp.  
`height` is the nonce of the block.  
`consensus_data` TBD  
`consensus_payload` TBD  
`next_book_keeper` TBD  

`book_keepers` contains the public keys of the accounts that signed this header  
`sig_data` contains the signatures  
`block_hash` the block hash, created only from hashing the first 9 fields.  

The init function does nothing, but the contract requires a post-deploy initialization, which is done by calling the syncGenesisHeader endpoint:
```
#[endpoint(syncGenesisHeader)]
fn sync_genesis_header(&self, header: Header) -> SCResult<()>
```

This will initialize the first header in the contract. Not much checking is done for this, so we "blindly" trust the genesis header.  

Checks will be done for every following header, which will be synchronized using the following endpoint:
```
#[endpoint(syncBlockHeader)]
fn sync_block_header(&self, header: Header) -> SCResult<()>
```

To be able to sync a new header, the new header has to be signed by at least 2/3 + 1 of the previous consensus group members.  

And that's about all this contract does! Its purpose is pretty simple, as the name suggests, it just synchronizes block headers.  

# EsdtTokenManager Smart Contract

This contract handles the wrapping/unwrapping of eGLD and the transfer of other wrapped tokens. 

This contract's init function requires the address of the CrossChainManagement contract. The details will be discussed when we detail the workflows, but for now, all you need to know is that certain endpoints in the EsdtTokenManager can only be called by the CrossChainManagement contract.

The EsdtTokenManager contract also requires some additional post-deploy initialization, which can only be performed by the owner. This is done by calling the following endpoint, which issues the tokens which will represent the wrapped eGLD and saves its identifier in the contract. This way, the contract itself is the owner of the wrapped eGLD and anyone can query the contract to see which ESDT is the "real" wrapped eGLD token representation.

```
#[payable]
#[endpoint(performWrappedEgldIssue)]
fn perform_wrapped_egld_issue(&self, initial_supply: BigUint, #[payment] payment: BigUint) -> SCResult<()>
```

This endpoint requires a payment of exactly 5 eGLD, which is the price for issueing an ESDT token at the time of writing this documentation (January 2021).  

## Wrapping and Unwrapping eGLD

Once the setup is complete, anyone can wrap and unwrap their eGLD using the following endpoints:

```
#[payable]
#[endpoint(wrapEgld)]
fn wrap_egld(&self, #[payment] payment: BigUint) -> SCResult<()>

#[endpoint(unwrapEgld)]
fn unwrap_egld(&self) -> SCResult<()>
```

The `wrap` endpoint accepts eGLD as payment and sends back the same amount of wrapped eGLD as ESDT, while `unwrap` accepts wrapped eGLD ESDT and sends back eGLD.  

## ESDT Operations

The contract can also perform other ESDT operations, specifically issue, mint and burn, although not all of them are available without restrictions. Only the contract owner may issue or burn tokens owned by the contract, but anyone may mint more tokens (although these tokens will still be locked in the contract). These are done through the following endpoints:  

### Owner-only

The owner may issue additional token types as more blockchains are linked to Elrond, using the following endpoint:

```
#[payable]
#[endpoint(issueEsdtToken)]
fn issue_esdt_token_endpoint(
    &self,
    token_display_name: BoxedBytes,
    token_ticker: BoxedBytes,
    initial_supply: BigUint,
    num_decimals: u8,
    #[payment] payment: BigUint,
) -> SCResult<()>
```

The process is similar to the one used for issueing the initial wrapped eGLD. The only difference is this endpoint provides more customization concerning the token's attributes. For wrapped eGLD, we use "Wrapped eGLD" as display name, "WEGLD" as ticker, and 18 decimals.  

The owner may also burn tokens owned by the smart contract if they deem it necessary (important: The owner _cannot_ burn tokens owned by other accounts, but these accounts may burn their tokens at will by directly sending the required transaction to the system smart contract).  

```
#[endpoint(burnEsdtToken)]
fn burn_esdt_token_endpoint(
    &self,
    token_identifier: BoxedBytes,
    amount: BigUint,
) -> SCResult<()>
```

### CrossChainManagement SC-only

The following endpoints may only be called by the CrossChainManagement contract. Theoretically, we could've had those functions in the CrossChainManagement SC directly, but this allows us to separate the logic of transactions and actual payments. We won't describe these in too much detail, as they won't make too much sense until we've described their role in the flow of execution.  

To perform a simple ESDTTransfer to an account, the following endpoint is used:  

```
#[endpoint(transferEsdtToAccount)]
fn transfer_esdt_to_account_endpoint(
    &self,
    token_identifier: BoxedBytes,
    amount: BigUint,
    to: Address,
    poly_tx_hash: H256,
) -> SCResult<()>
```

`poly_tx_hash` is the hash of the cross-chain transaction. We need this to be able to mark transactions as executed (so the same transaction can't somehow be executed twice).  One thing to note about this and the following endpoint is that if the token is wrapped eGLD, they will automatically be unwrapped before being sent over.  

To perform a scCall with ESDT as payment, the following endpoint is called:

```
#[endpoint(transferEsdtToContract)]
fn transfer_esdt_to_contract_endpoint(
    &self,
    token_identifier: BoxedBytes,
    amount: BigUint,
    to: Address,
    func_name: BoxedBytes,
    args: Vec<BoxedBytes>,
    poly_tx_hash: H256,
) -> SCResult<()>
```

The arguments are the same as the ones for the simple transfer, except we also have a function name and the respective arguments.  

# CrossChainManagement Smart Contract

This is the main contract, responsible for performing validity checks for transactions, and basically dictates the whole workflow. Because of that, we're going to first describe the basic endpoints and then we're going to describe the workflow, describing each endpoint as we go.  

Its init endpoint looks like follows:
```
#[init]
fn init(&self, header_sync_contract_address: Address, own_chain_id: u64)
```

## Owner-only

The contract requires the address of the HeaderSyncContract and its own chain id, which is the id which was assigned by the cross chain intermediary to our chain.  

It also requires the address of the EsdtTokenManager contract, but as that contract also requires the address of _this_ contract, we would have a circular dependency, so we set it after deploy, using the following owner-only endpoint:  

```
#[endpoint(setTokenManagementContractAddress)]
fn set_token_management_contract_address_endpoint(&self, address: Address) -> SCResult<()>
```

It's also worth noting that not every ESDT token is accepted. The contract keeps track of a whitelist of tokens that can only be altered by the contract owner. Adding or removing is done by calling the following endpoints respectively:  

```
#[endpoint(addTokenToWhitelist)]
fn add_token_to_whitelist(&self, token_identifier: BoxedBytes) -> SCResult<()>
```

```
#[endpoint(removeTokenFromWhitelist)]
fn remove_token_from_whitelist(&self, token_identifier: BoxedBytes) -> SCResult<()>
```

# Workflows

There are two general workflows that we have to go through: Receiving a transaction _from_ another chain, and sending a transaction _to_ another chain. 

## Transaction - Send

We'll start with sending a transaction, as that one is pretty straight-forward. Anyone can call the following endpoint to initiate a cross chain transaction:

```
#[endpoint(createCrossChainTx)]
fn create_cross_chain_tx(
    &self,
    to_chain_id: u64,
    to_contract_address: Address,
    method_name: BoxedBytes,
    method_args: Vec<BoxedBytes>,
) -> SCResult<()>
```

The caller may also deposit a number of ESDT tokens to be used as payment. If the token is not on the whitelist, the transaction will be rejected.

The transaction will be saved and processed later by the cross chain intermediary. And that's all!

## Transaction - Receive

Due to some limitations, we currently can't nest mulitple async-calls, so this flow will require multiple steps to reach completion. In the image below, we have the workflow, split into 3 main parts. First are the numbered steps, second are the one noted with lowercase letters, and finally, the ones noted with uppercase letters.

[TBD]