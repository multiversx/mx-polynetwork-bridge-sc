### Common

# update after token manager deploy, "0x" followed by the "hex" part
WRAPPED_EGLD_TOKEN_IDENTIFIER=0x5745474c442d333135356638

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
    issueWrappedEgld 0x32 # 50 wrapped eGLD issue
    storeIncrementNonce

    sleep 40

    echo "Wrapped eGLD token identifier:"
    getWrappedEgldTokenIdentifier
}

queryEsdtTokenManager() {
    source ../EsdtTokenManager/interaction/snippets.sh

    echo "Wrapped eGLD token identifier:"
    getWrappedEgldTokenIdentifier

    echo "Was wrapped eGLD issued? (1 = true, 0 = false)"
    wasTokenIssued ${WRAPPED_EGLD_TOKEN_IDENTIFIER}

    echo "Total wrapped eGLD:"
    getTokenAmount ${WRAPPED_EGLD_TOKEN_IDENTIFIER}
}

mintMoreWrappedEgld() {
    source ../EsdtTokenManager/interaction/snippets.sh

    loadNonce
    mintTokens ${WRAPPED_EGLD_TOKEN_IDENTIFIER} 0x0A
    storeIncrementNonce

    sleep 40

    echo "Total wrapped eGLD:"
    getTokenAmount ${WRAPPED_EGLD_TOKEN_IDENTIFIER}
}

burnWrappedEgld() {
    source ../EsdtTokenManager/interaction/snippets.sh

    loadNonce
    burnTokens ${WRAPPED_EGLD_TOKEN_IDENTIFIER} 0x05
    storeIncrementNonce

    sleep 40

    echo "Total wrapped eGLD:"
    getTokenAmount ${WRAPPED_EGLD_TOKEN_IDENTIFIER}
}

### Scnearios

# Scenario 1

# Alice wraps 10 eGLD and sends to an offchain account

# Scenario 2

# Alice receives 6 wrapped ETH
# Alice sends 3 wrapped ETH to Bob (3 ETH left)
# Alice sends 2 wrapped ETH to an offchain account (1 ETH left)
