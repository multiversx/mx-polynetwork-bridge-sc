### Common

loadNonce() {
    alice_nonce=$(erdpy data load --key=alice_nonce)
    alice_nonce="${alice_nonce:-5}"
}

storeIncrementNonce() {
    alice_nonce="${alice_nonce:-5}"
    erdpy data store --key=alice_nonce --value=$((alice_nonce + 1))
}

### BlockHeaderSync setup. It currently doesn't do any checking, so it's pretty much a mock at this point.

deployAndSetupBlockHeaderSync() {
    loadNonce
    source ../BlockHeaderSync/interaction/snippets.sh
    deploy
    storeIncrementNonce

    sleep 10

    loadNonce
    syncGenesisHeader
    storeIncrementNonce

    sleep 10

    getHeaderByHeight
}

deployCrossChainManagement() {
    loadNonce
    source ../CrossChainManagement/interaction/snippets.sh
    deploy
    storeIncrementNonce
}

deployEsdtTokenManager() {
    source ../EsdtTokenManager/interaction/snippets.sh
    
    loadNonce
    deploy
    storeIncrementNonce

    sleep 10

    loadNonce
    issueWrappedEgld
    storeIncrementNonce

    sleep 40

    getWrappedEgldTokenIdentifier
}

getWrappedEgldTokenIdentifierExternal()
{
    source ../EsdtTokenManager/interaction/snippets.sh
    getWrappedEgldTokenIdentifier
}


### Scnearios

# Alice receives 6 wrapped ETH
# Alice sends 3 wrapped ETH to Bob (3 ETH left)
# Alice sends 2 wrapped ETH to an offchain account (1 ETH left)

# Alice wraps 10 eGLD and sends to an offchain account