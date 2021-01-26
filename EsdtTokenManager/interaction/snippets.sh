ALICE="/home/elrond/elrond-sdk/erdpy/testnet/wallets/users/alice.pem"
ADDRESS=$(erdpy data load --key=address-testnet-esdtTokenManager)
DEPLOY_TRANSACTION=$(erdpy data load --key=deployTransaction-testnet-esdtTokenManager)
PROXY=http://localhost:7950 # For public testnet, replace with https://testnet-gateway.elrond.com
CHAIN_ID=local-testnet
PROJECT_HARDCODED="/home/elrond/sc-polynetwork-bridge-rs/EsdtTokenManager"

CROSS_CHAIN_MANAGEMENT_ADDRESS=$(erdpy data load --key=address-testnet-crossChainManagement)
CROSS_CHAIN_MANAGEMENT_ADDRESS_DECODED=$(erdpy wallet bech32 --decode ${CROSS_CHAIN_MANAGEMENT_ADDRESS})

# To get tx result, go to http://localhost:7950/transaction/tx_hash_here?withResults=true

deploy() {
    erdpy --verbose contract deploy --project=${PROJECT_HARDCODED} --nonce=${alice_nonce} --pem=${ALICE} --gas-limit=200000000 --arguments 0x${CROSS_CHAIN_MANAGEMENT_ADDRESS_DECODED} --send --outfile="deploy-testnet.interaction.json" --proxy=${PROXY} --chain=${CHAIN_ID} || return

    TRANSACTION=$(erdpy data parse --file="deploy-testnet.interaction.json" --expression="data['emitted_tx']['hash']")
    ADDRESS=$(erdpy data parse --file="deploy-testnet.interaction.json" --expression="data['emitted_tx']['address']")

    erdpy data store --key=address-testnet-esdtTokenManager --value=${ADDRESS}
    erdpy data store --key=deployTransaction-testnet-esdtTokenManager --value=${TRANSACTION}

    echo ""
    echo "Smart contract address: ${ADDRESS}"
}

issueWrappedEgld() {
    erdpy --verbose contract call ${ADDRESS} --nonce=${alice_nonce} --pem=${ALICE} --gas-limit=2000000000 --function="performWrappedEgldIssue" --value=5000000000000000000 --arguments 0x05 --send --proxy=${PROXY} --chain=${CHAIN_ID}
}

getWrappedEgldTokenIdentifier() {
    #local as_hex=$(
    erdpy --verbose contract query ${ADDRESS} --function="getWrappedEgldTokenIdentifier" --proxy=${PROXY} #| jq '.hex')
    #echo "$as_hex"
}

getErrCode() {
    erdpy --verbose contract query ${ADDRESS} --function="getErrCode" --proxy=${PROXY}
}
