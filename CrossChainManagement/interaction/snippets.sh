ALICE="/home/elrond/elrond-sdk/erdpy/testnet/wallets/users/alice.pem"
# After deploy, replace with the assigned address
ADDRESS=$(erdpy data load --key=address-testnet)
DEPLOY_TRANSACTION=$(erdpy data load --key=deployTransaction-testnet)
PROXY=http://localhost:7950 # For public testnet, replace with https://testnet-gateway.elrond.com
CHAIN_ID=local-testnet

# Deploy BlockHeaderSync SC first, then add its address here
BLOCK_HEADER_SYNC_ADDRESS=
# Doesn't really matter what you input here, as long as it fits in a u64. We'll just use the magic meaning of life number: 42
OWN_CHAIN_ID=0x000000000000002A

# To get tx result, go to http://localhost:7950/transaction/tx_hash_here?withResults=true

deploy() {
    erdpy --verbose contract deploy --project=${PROJECT} --recall-nonce --pem=${ALICE} --gas-limit=200000000 --arguments ${BLOCK_HEADER_SYNC_ADDRESS} ${OWN_CHAIN_ID} --send --outfile="deploy-testnet.interaction.json" --proxy=${PROXY} --chain=${CHAIN_ID} || return

    TRANSACTION=$(erdpy data parse --file="deploy-testnet.interaction.json" --expression="data['emitted_tx']['hash']")
    ADDRESS=$(erdpy data parse --file="deploy-testnet.interaction.json" --expression="data['emitted_tx']['address']")

    erdpy data store --key=address-testnet --value=${ADDRESS}
    erdpy data store --key=deployTransaction-testnet --value=${TRANSACTION}

    echo ""
    echo "Smart contract address: ${ADDRESS}"
}
