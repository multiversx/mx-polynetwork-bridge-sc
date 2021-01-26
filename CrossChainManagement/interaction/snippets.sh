ALICE="/home/elrond/elrond-sdk/erdpy/testnet/wallets/users/alice.pem"
ADDRESS=$(erdpy data load --key=address-testnet-crossChainManagement)
DEPLOY_TRANSACTION=$(erdpy data load --key=deployTransaction-testnet-crossChainManagement)
PROXY=http://localhost:7950 # For public testnet, replace with https://testnet-gateway.elrond.com
CHAIN_ID=local-testnet
PROJECT_HARDCODED="/home/elrond/sc-polynetwork-bridge-rs/CrossChainManagement"

BLOCK_HEADER_SYNC_ADDRESS=$(erdpy data load --key=address-testnet-blockHeaderSync)
BLOCK_HEADER_SYNC_ADDRESS_DECODED=$(erdpy wallet bech32 --decode ${BLOCK_HEADER_SYNC_ADDRESS})
# Doesn't really matter what you input here, as long as it fits in a u64. We'll just use the magic meaning of life number: 42
OWN_CHAIN_ID=0x000000000000002A

ESDT_TOKEN_MANAGER_ADDRESS=$(erdpy data load --key=address-testnet-esdtTokenManager)
ESDT_TOKEN_MANAGER_ADDRESS_DECODED=$(erdpy wallet bech32 --decode ${ESDT_TOKEN_MANAGER_ADDRESS})

# To get tx result, go to http://localhost:7950/transaction/tx_hash_here?withResults=true

deploy() {
    erdpy --verbose contract deploy --project=${PROJECT_HARDCODED} --nonce=${alice_nonce} --pem=${ALICE} --gas-limit=200000000 --arguments 0x${BLOCK_HEADER_SYNC_ADDRESS_DECODED} ${OWN_CHAIN_ID} --send --outfile="deploy-testnet.interaction.json" --proxy=${PROXY} --chain=${CHAIN_ID} || return

    TRANSACTION=$(erdpy data parse --file="deploy-testnet.interaction.json" --expression="data['emitted_tx']['hash']")
    ADDRESS=$(erdpy data parse --file="deploy-testnet.interaction.json" --expression="data['emitted_tx']['address']")

    erdpy data store --key=address-testnet-crossChainManagement --value=${ADDRESS}
    erdpy data store --key=deployTransaction-testnet-crossChainManagement --value=${TRANSACTION}

    echo ""
    echo "Smart contract address: ${ADDRESS}"
}

setTokenManagerAddress() {
    erdpy --verbose contract call ${ADDRESS} --nonce=${alice_nonce} --pem=${ALICE} --gas-limit=50000000 --function="setTokenManagementContractAddress" --arguments 0x${ESDT_TOKEN_MANAGER_ADDRESS_DECODED} --send --proxy=${PROXY} --chain=${CHAIN_ID}
}

# token name received as argument $1
addTokenToWhitelist() {
    erdpy --verbose contract call ${ADDRESS} --nonce=${alice_nonce} --pem=${ALICE} --gas-limit=50000000 --function="addTokenToWhitelist" --arguments $1 --send --proxy=${PROXY} --chain=${CHAIN_ID}
}