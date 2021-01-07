
#![no_std]

imports!();

use elrond_wasm::ArgBuffer;
use header::*;
use transaction::*;

#[elrond_wasm_derive::callable(BlockHeaderSyncProxy)]
pub trait BlockHeaderSync {
	#[rustfmt::skip]
	#[callback(get_header_by_height_callback)]
    fn getHeaderByHeight(&self, chain_id: u64, height: u32, #[callback_arg] cb_tx: &Transaction);
}

#[elrond_wasm_derive::contract(CrossChainManagementImpl)]
pub trait CrossChainManagement {
    #[init]
    fn init(&self, header_sync_contract_address: &Address, own_chain_id: u64) {
        self.set_header_sync_contract_address(header_sync_contract_address);
        self.set_own_chain_id(own_chain_id);
    }

    // endpoints

    #[payable]
    #[endpoint(createCrossChainTx)]
    fn create_cross_chain_tx(&self, to_chain_id: u64, to_contract_address: Address, 
        method_name: BoxedBytes, method_args: Vec<BoxedBytes>, #[payment] payment: BigUint)
        -> SCResult<()> {

        let from_contract_address = self.get_caller();

        let mut locked_funds = self.get_locked_funds(&from_contract_address);
        locked_funds += payment;
        self.set_locked_funds(&from_contract_address, &locked_funds);
        
        let mut tx = Transaction {
            tx_hash: H256::zero(),
            tx_id: self.get_cross_chain_tx_id(to_chain_id),
            from_contract_address,
            to_chain_id,
            to_contract_address,
            method_name,
            method_args,
        };
        tx.tx_hash = self.hash_transaction(&tx);

        self.set_tx_by_id(to_chain_id, tx.tx_id, &tx);
        self.set_cross_chain_tx_id(to_chain_id, tx.tx_id + 1);

        self.create_tx_event(&tx);

        Ok(())
    }

    #[endpoint(processCrossChainTx)]
    fn process_cross_chain_tx(&self, from_chain_id: u64, height: u32, tx: Transaction) -> SCResult<()> {
        require!(self.get_own_chain_id() == tx.to_chain_id, "This transaction is meant for another chain!");

		let contract_address = self.get_header_sync_contract_address();
		let proxy = contract_proxy!(self, &contract_address, BlockHeaderSync);
        proxy.getHeaderByHeight(from_chain_id, height, &tx);
        
        Ok(())
    }

    // callbacks

    #[callback]
	fn get_header_by_height_callback(
		&self,
		result: AsyncCallResult<Option<Header>>,
		#[callback_arg] cb_tx: Transaction
	) {
		match result {
			AsyncCallResult::Ok(opt_header) => {
				match opt_header {
                    Some(_header) => {
                        // TODO: check tx proof

                        // pack arguments
                        let mut arg_buffer = ArgBuffer::new();
                        for arg in &cb_tx.method_args {
                            arg_buffer.push_raw_arg(arg.as_slice());
                        }

                        // TO DO: Handle payable functions
                        
                        // execute scCall
                        self.execute_on_dest_context(
                            self.get_gas_left(),
                            &cb_tx.to_contract_address, 
                            &BigUint::zero(), 
                            cb_tx.method_name.as_slice(),
                            &arg_buffer);
                    },
                    None => {
                        // could not find header
                        // should sync header first
                    }
                };
			},
			AsyncCallResult::Err(_) => {},
		}
    }
    
    // private

    fn hash_transaction(&self, tx: &Transaction) -> H256 {
        self.sha256(tx.get_partial_serialized().as_slice())
    }

    // events 

    #[event("0x1000000000000000000000000000000000000000000000000000000000000001")]
    fn create_tx_event(&self, tx: &Transaction);

    // storage

    // header sync contract address

    #[storage_get("headerSyncContractAddress")]
    fn get_header_sync_contract_address(&self) -> Address;

    #[storage_set("headerSyncContractAddress")]
    fn set_header_sync_contract_address(&self, address: &Address);

    // locked funds

    #[view(getLockedFunds)]
    #[storage_get("lockedFunds")]
    fn get_locked_funds(&self, address: &Address) -> BigUint;

    #[storage_set("lockedFunds")]
    fn set_locked_funds(&self, address: &Address, amount: &BigUint);

    // own chain id

    #[view(getOwnChainId)]
    #[storage_get("ownChainId")]
    fn get_own_chain_id(&self) -> u64;

    #[storage_set("ownChainId")]
    fn set_own_chain_id(&self, own_chain_id: u64);

    // cross chain tx id

    #[storage_get("crossChainTxId")]
    fn get_cross_chain_tx_id(&self, chain_id: u64) -> u64;

    #[storage_set("crossChainTxId")]
    fn set_cross_chain_tx_id(&self, chain_id: u64, tx_id: u64);

    // tx by id

    #[view(getTxById)]
    #[storage_get("txById")]
    fn get_tx_by_id(&self, chain_id: u64, tx_id: u64) -> Transaction;

    #[storage_set("txById")]
    fn set_tx_by_id(&self, chain_id: u64, tx_id: u64, tx: &Transaction);
}
