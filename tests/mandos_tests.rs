extern crate block_header_sync;
use block_header_sync::*;

extern crate cross_chain_management;
use cross_chain_management::*;

extern crate esdt_token_manager;
use esdt_token_manager::*;

use elrond_wasm::*;
use elrond_wasm_debug::*;

fn contract_map() -> ContractMap<TxContext> {
	let mut contract_map = ContractMap::new();

	contract_map.register_contract(
		"file:../BlockHeaderSync/output/block-header-sync.wasm",
		Box::new(|context| Box::new(BlockHeaderSyncImpl::new(context))),
	);

	contract_map.register_contract(
		"file:../CrossChainManagement/output/cross-chain-management.wasm",
		Box::new(|context| Box::new(CrossChainManagementImpl::new(context))),
	);

	contract_map.register_contract(
		"file:../EsdtTokenManager/output/esdt-token-manager.wasm",
		Box::new(|context| Box::new(EsdtTokenManagerImpl::new(context))),
	);

	contract_map
}

#[test]
fn setup() {
	parse_execute_mandos(
		"mandos/setup.scen.json",
		&contract_map(),
	);
}
