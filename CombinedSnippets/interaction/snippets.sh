### Common

# update after token manager deploy, "0x" followed by the "hex" part
WRAPPED_EGLD_TOKEN_IDENTIFIER=0x5745474c442d643736326565

# update after issue
WRAPPED_ETH_TOKEN_IDENTIFIER=0x574554482d333365313865

# No need to update this, as it's always the same poly tx, which in turn means the same hash
# Hash for TX from Elrond to another chain
FROM_ERD_TX_HASH=0xd95c06a936c765969c42846432d41268fd73c7a169e10ad1543050a4431edb04

# No need to update, always the same
# Tx from Ethereum (just an example, could be any chain) to Elrond
FROM_ETH_TRANSACTION=0x25a4fa887af0bb300e21a4bf8c6a7101a17c2039af36ae9b33b32ee962e64039000000000000000000000000000000000000000000000000000000000000000000000000000000002a000000000000000139472eff6886771a982f3083da5d421f24c29181e63888228dc81ca60d69e10000
FROM_ETH_TX_HASH=0x25a4fa887af0bb300e21a4bf8c6a7101a17c2039af36ae9b33b32ee962e64039

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

    getTxByHash ${FROM_ERD_TX_HASH}
}

getPaymentForTransaction() {
    source ../CrossChainManagement/interaction/snippets.sh

    getPaymentForTx ${FROM_ERD_TX_HASH}
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

# Issue wrapped ETH and add to token whitelist
# Alice receives 6 wrapped ETH
# Alice sends 2 wrapped ETH to an offchain account (4 ETH left)

issueWrappedEth() {
    source ../EsdtTokenManager/interaction/snippets.sh

    loadNonce
    issueToken 0x57726170706564455448 0x57455448 0x0A 0x00
    storeIncrementNonce

    sleep 40

    echo "Wrapped ETH token identifier:"
    getLastIssuedTokenIdentifier
}

addWrappedEthToWhitelist() {
    source ../CrossChainManagement/interaction/snippets.sh

    loadNonce
    addTokenToWhitelist ${WRAPPED_ETH_TOKEN_IDENTIFIER}
    storeIncrementNonce
}

getTotalWrappedEth() {
    source ../EsdtTokenManager/interaction/snippets.sh

    echo "Total wrapped ETH:"
    getTokenAmount ${WRAPPED_ETH_TOKEN_IDENTIFIER}
}

receiveSixWrappedEth() {
    source ../CrossChainManagement/interaction/snippets.sh

    loadNonce
    processCrossChainTx 0x00 0x00 ${FROM_ETH_TRANSACTION} ${WRAPPED_ETH_TOKEN_IDENTIFIER} 0x06
    storeIncrementNonce
}

getPayment() {
    source ../CrossChainManagement/interaction/snippets.sh

    echo "Payment for tx:"
    getPaymentForTx 0x25a4fa887af0bb300e21a4bf8c6a7101a17c2039af36ae9b33b32ee962e64039
}

processReceivedEthTx() {
    source ../CrossChainManagement/interaction/snippets.sh

    loadNonce
    processPendingTx ${FROM_ETH_TX_HASH}
    storeIncrementNonce
}

getTransactionStatus() {
    source ../CrossChainManagement/interaction/snippets.sh

    getTxStatus ${FROM_ETH_TX_HASH}
}
