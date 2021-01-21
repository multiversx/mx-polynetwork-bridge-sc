ALICE="/home/elrond/elrond-sdk/erdpy/testnet/wallets/users/alice.pem"
# After deploy, replace with the assigned address
ADDRESS=$(erdpy data load --key=address-testnet)
DEPLOY_TRANSACTION=$(erdpy data load --key=deployTransaction-testnet)
PROXY=http://localhost:7950 # For public testnet, replace with https://testnet-gateway.elrond.com
CHAIN_ID=local-testnet

# To get tx result, go to http://localhost:7950/transaction/tx_hash_here?withResults=true

deploy() {
    erdpy --verbose contract deploy --project=${PROJECT} --recall-nonce --pem=${ALICE} --gas-limit=200000000 --send --outfile="deploy-testnet.interaction.json" --proxy=${PROXY} --chain=${CHAIN_ID} || return

    TRANSACTION=$(erdpy data parse --file="deploy-testnet.interaction.json" --expression="data['emitted_tx']['hash']")
    ADDRESS=$(erdpy data parse --file="deploy-testnet.interaction.json" --expression="data['emitted_tx']['address']")

    erdpy data store --key=address-testnet --value=${ADDRESS}
    erdpy data store --key=deployTransaction-testnet --value=${TRANSACTION}

    echo ""
    echo "Smart contract address: ${ADDRESS}"
}

syncGenesisHeader() {
    erdpy --verbose contract call ${ADDRESS} --recall-nonce --pem=${ALICE} --gas-limit=100000000 --function="syncGenesisHeader" --arguments 0x00000001000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000 --send --proxy=${PROXY} --chain=${CHAIN_ID}
}

syncBlockHeader() {
    erdpy --verbose contract call ${ADDRESS} --recall-nonce --pem=${ALICE} --gas-limit=10000000 --function="syncBlockHeader" --arguments 0x00000001000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000 --send --proxy=${PROXY} --chain=${CHAIN_ID}
}

verifyHeader() {
    erdpy --verbose contract call ${ADDRESS} --recall-nonce --pem=${ALICE} --gas-limit=10000000 --function="verifyHeader" --arguments 0x00000001000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000 --send --proxy=${PROXY} --chain=${CHAIN_ID}
}

getHeaderByHeight() {
    erdpy --verbose contract query ${ADDRESS} --function="getHeaderByHeight" --arguments 0x0000000000000000 0x00000000 --proxy=${PROXY}
}

### Scenario ###
# deploy()
# hardcode the address in the ADDRESS variable
# syncGenesisHeader()
# getHeaderByHeight()
