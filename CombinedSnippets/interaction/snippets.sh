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

deployBlockHeaderSync() {
    loadNonce
    source ../BlockHeaderSync/interaction/snippets.sh
    deploy
    storeIncrementNonce
}

setupBlockHeaderSync() {
    loadNonce
    source ../BlockHeaderSync/interaction/snippets.sh
    syncGenesisHeader
    storeIncrementNonce

    loadNonce
    source ../BlockHeaderSync/interaction/snippets.sh
    syncBlockHeader
    storeIncrementNonce
}

getHeaderByHeightExternal() {
    source ../BlockHeaderSync/interaction/snippets.sh
    getHeaderByHeight
}


deployCrossChainManagement() {
    loadNonce
    source ../CrossChainManagement/interaction/snippets.sh
    deploy
    storeIncrementNonce
}

deployEsdtTokenManager() {
    loadNonce
    source ../EsdtTokenManager/interaction/snippets.sh
    deploy
    storeIncrementNonce
}

setupCrossChainManagementAndEsdtTokenManager() {
    loadNonce
    source ../CrossChainManagement/interaction/snippets.sh
    setTokenManagerAddress
    storeIncrementNonce

    loadNonce
    source ../EsdtTokenManager/interaction/snippets.sh
    issueWrappedEgld
    storeIncrementNonce
}

getWrappedEgldTokenIdentifierExternal() {
    source ../EsdtTokenManager/interaction/snippets.sh
    #echo "Wrapped eGLD token identifier:"
    #echo $(getWrappedEgldTokenIdentifier)
    getWrappedEgldTokenIdentifier
}

getErrCodeExternal() {
    source ../EsdtTokenManager/interaction/snippets.sh
    getErrCode
}

### Scnearios

# Alice receives 6 wrapped ETH
# Alice sends 3 wrapped ETH to Bob (3 ETH left)
# Alice sends 2 wrapped ETH to an offchain account (1 ETH left)

# Alice wraps 10 eGLD and sends to an offchain account