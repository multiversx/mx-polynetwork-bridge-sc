use elrond_wasm::io::EndpointResult;
use elrond_wasm::types::{AsyncCall, SendEgld, SendEsdt};
use elrond_wasm::{
    api::{BigUintApi, EndpointFinishApi, ErrorApi, SendApi},
    types::TransferEgldExecute,
};

elrond_wasm::derive_imports!();

#[derive(TypeAbi)]
pub enum TransferEsdtActionResult<BigUint: BigUintApi> {
    Nothing,
    SendEgld(SendEgld<BigUint>),
    SendEsdt(SendEsdt<BigUint>),
    TransferEgldExecute(TransferEgldExecute<BigUint>),
    AsyncCall(AsyncCall<BigUint>),
}

impl<FA, BigUint> EndpointResult<FA> for TransferEsdtActionResult<BigUint>
where
    BigUint: BigUintApi + 'static,
    FA: EndpointFinishApi + ErrorApi + SendApi<BigUint> + Clone + 'static,
{
    fn finish(&self, api: FA) {
        match self {
            TransferEsdtActionResult::Nothing => (),
            TransferEsdtActionResult::SendEgld(send_egld) => send_egld.finish(api),
            TransferEsdtActionResult::SendEsdt(send_esdt) => send_esdt.finish(api),
            TransferEsdtActionResult::TransferEgldExecute(transfer_egld_execute) => {
                transfer_egld_execute.finish(api)
            }
            TransferEsdtActionResult::AsyncCall(async_call) => async_call.finish(api),
        }
    }
}
