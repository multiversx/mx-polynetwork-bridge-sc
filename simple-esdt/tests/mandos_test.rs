extern crate simple_esdt;
use simple_esdt::*;

use elrond_wasm::*;
use elrond_wasm_debug::*;

fn _contract_map() -> ContractMap<TxContext> {
	let mut contract_map = ContractMap::new();
	contract_map.register_contract(
		"file:../output/simple-esdt.wasm",
		Box::new(|context| Box::new(SimpleEsdtImpl::new(context))),
	);
	contract_map
}
