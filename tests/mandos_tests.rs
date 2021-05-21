extern crate block_header_sync;
extern crate cross_chain_management;

use elrond_wasm::*;
use elrond_wasm_debug::*;

fn contract_map() -> ContractMap<TxContext> {
	let mut contract_map = ContractMap::new();

	contract_map.register_contract(
		"file:../BlockHeaderSync/output/block-header-sync.wasm",
		Box::new(|context| Box::new(block_header_sync::contract_obj(context))),
	);

	contract_map.register_contract(
		"file:../CrossChainManagement/output/cross-chain-management.wasm",
		Box::new(|context| Box::new(cross_chain_management::contract_obj(context))),
	);

	contract_map
}

#[test]
fn deploy() {
	parse_execute_mandos(
		"mandos/deploy.scen.json",
		&contract_map(),
	);
}
