extern crate block_header_sync;
use block_header_sync::*;
use elrond_wasm::*;
use elrond_wasm_debug::*;

fn contract_map() -> ContractMap<TxContext> {
    let mut contract_map = ContractMap::new();
    contract_map.register_contract(
        "file:../output/block-header-sync.wasm",
        Box::new(|context| Box::new(BlockHeaderSyncImpl::new(context))),
    );
    contract_map
}

#[test]
fn test_mandos() {
    parse_execute_mandos("mandos/.scen.json", &contract_map());
}
