extern crate simple_erc20;
use simple_erc20::*;

use elrond_wasm::*;
use elrond_wasm_debug::*;

fn _contract_map() -> ContractMap<TxContext> {
	let mut contract_map = ContractMap::new();
	contract_map.register_contract(
		"file:../output/simple-erc20.wasm",
		Box::new(|context| Box::new(SimpleErc20TokenImpl::new(context))),
	);
	contract_map
}
