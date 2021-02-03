### Common

# update after token manager deploy, "0x" followed by the "hex" part
WRAPPED_EGLD_TOKEN_IDENTIFIER=0x5745474c442d653162653036

# update after getting next pending tx
POLY_TX_HASH=0x00

loadNonce() {
    alice_nonce=$(erdpy data load --key=alice_nonce)
    alice_nonce="${alice_nonce:-5}"
}

storeIncrementNonce() {
    alice_nonce="${alice_nonce:-5}"
    erdpy data store --key=alice_nonce --value=$((alice_nonce + 1))
}

# SETUP - To be done in this exact order

deployAndSetupBlockHeaderSync() {
    source ../BlockHeaderSync/interaction/snippets.sh

    loadNonce
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
    source ../CrossChainManagement/interaction/snippets.sh

    loadNonce
    deploy
    storeIncrementNonce
}

# Remember to update WRAPPED_EGLD_TOKEN_IDENTIFIER variable after deployment is complete
deployEsdtTokenManager() {
    source ../EsdtTokenManager/interaction/snippets.sh
    
    loadNonce
    deploy
    storeIncrementNonce

    sleep 10

    loadNonce
    issueWrappedEgld 0x02B5E3AF16B1880000 # 50 wrapped eGLD issue (50 * 10^18)
    storeIncrementNonce

    sleep 40

    echo "Wrapped eGLD token identifier:"
    getWrappedEgldTokenIdentifier
}

finalizeSetup() {
    source ../CrossChainManagement/interaction/snippets.sh

    loadNonce
    setTokenManagerAddress
    storeIncrementNonce

    sleep 10

    loadNonce
    addTokenToWhitelist ${WRAPPED_EGLD_TOKEN_IDENTIFIER}
    storeIncrementNonce

    sleep 10

    loadNonce
    addAddressToApprovedlist 0x0139472eff6886771a982f3083da5d421f24c29181e63888228dc81ca60d69e1 # Alice's address
    storeIncrementNonce
}

### Test functions. Can be called in any order to test particular functionalities.

# Esdt Token Manager

queryEsdtTokenManager() {
    source ../EsdtTokenManager/interaction/snippets.sh

    echo "Wrapped eGLD token identifier:"
    getWrappedEgldTokenIdentifier

    echo "Total wrapped eGLD:"
    getTokenAmount ${WRAPPED_EGLD_TOKEN_IDENTIFIER}

    echo "Total locked eGLD:"
    getLockedEgldBalance
}

mintMoreWrappedEgld() {
    source ../EsdtTokenManager/interaction/snippets.sh

    loadNonce
    mintTokens ${WRAPPED_EGLD_TOKEN_IDENTIFIER} 0x8AC7230489E80000 # 10 wrapped eGLD (10 * 10^18)
    storeIncrementNonce

    sleep 40

    echo "Total wrapped eGLD:"
    getTokenAmount ${WRAPPED_EGLD_TOKEN_IDENTIFIER}
}

burnWrappedEgld() {
    source ../EsdtTokenManager/interaction/snippets.sh

    loadNonce
    burnTokens ${WRAPPED_EGLD_TOKEN_IDENTIFIER} 0x8AC7230489E80000 # 10 wrapped eGLD (10 * 10^18)
    storeIncrementNonce

    sleep 40

    echo "Total wrapped eGLD:"
    getTokenAmount ${WRAPPED_EGLD_TOKEN_IDENTIFIER}
}

# Cross Chain Management

getNextPendingCrossChainTransation() {
    source ../CrossChainManagement/interaction/snippets.sh

    loadNonce
    getNextPendingTx
    storeIncrementNonce
}

getTransactionByHash() {
    source ../CrossChainManagement/interaction/snippets.sh

    loadNonce
    getTxByHash ${POLY_TX_HASH}
    storeIncrementNonce
}

### Scnearios

# Scenario 1

# Alice wraps 10 eGLD
# Alice unwraps 5 eGLD
# Alice sends the remaining 5 wrapped eGLD to an offchain account

wrapTenEgld() {
    source ../EsdtTokenManager/interaction/snippets.sh

    loadNonce
    wrapEgld 10000000000000000000 # 10 * 10^18
    storeIncrementNonce

    sleep 20

    queryEsdtTokenManager
}

unwrapFiveEgld() {
    source ../EsdtTokenManager/interaction/snippets.sh

    loadNonce
    unwrapEgld ${WRAPPED_EGLD_TOKEN_IDENTIFIER} 0x4563918244F40000 # 5 * 10^18
    storeIncrementNonce

    sleep 20

    queryEsdtTokenManager
}

sendFiveEgldToAnotherChain() {
    source ../CrossChainManagement/interaction/snippets.sh

    loadNonce
    createCrossChainTx ${WRAPPED_EGLD_TOKEN_IDENTIFIER} 0x4563918244F40000 0x0A 0x0139472eff6886771a982f3083da5d421f24c29181e63888228dc81ca60d69e1 0x 0x  # 5 wrapped eGLD (5 * 10^18)
    storeIncrementNonce
}

# Scenario 2

# Alice receives 6 wrapped ETH
# Alice sends 3 wrapped ETH to Bob (3 ETH left)
# Alice sends 2 wrapped ETH to an offchain account (1 ETH left)
