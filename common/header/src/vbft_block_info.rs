use elrond_wasm::elrond_codec::*;
use elrond_wasm::types::BoxedBytes;

use zero_copy_sink::*;
use zero_copy_source::*;

use super::chain_config::*;

elrond_wasm::derive_imports!();

#[derive(TypeAbi, Debug, PartialEq)]
pub struct VbftBlockInfo {
    pub proposer: u32,
    pub vrf_value: BoxedBytes, // TBD: Discuss
    pub vrf_proof: BoxedBytes, // TBD: Discuss
    pub last_config_block_num: u32,
    pub new_chain_config: Option<ChainConfig>,
}

impl VbftBlockInfo {
    pub fn decode_from_source(
        source: &mut ZeroCopySource,
        has_new_chain_config: bool,
    ) -> Result<Self, DecodeError> {
        let proposer;
        let vrf_value;
        let vrf_proof;
        let last_config_block_num;
        let new_chain_config;

        match source.next_u32() {
            Some(val) => proposer = val,
            None => return Err(DecodeError::INPUT_TOO_SHORT),
        };

        match source.next_var_bytes() {
            Some(val) => vrf_value = val,
            None => return Err(DecodeError::INPUT_TOO_SHORT),
        };

        match source.next_var_bytes() {
            Some(val) => vrf_proof = val,
            None => return Err(DecodeError::INPUT_TOO_SHORT),
        };

        match source.next_u32() {
            Some(val) => last_config_block_num = val,
            None => return Err(DecodeError::INPUT_TOO_SHORT),
        };

        if has_new_chain_config {
            match ChainConfig::decode_from_source(source) {
                Ok(config) => new_chain_config = Some(config),
                Err(err) => return Err(err),
            };
        } else {
            new_chain_config = None;
        }

        return Ok(VbftBlockInfo {
            proposer,
            vrf_value,
            vrf_proof,
            last_config_block_num,
            new_chain_config,
        });
    }
}

impl NestedEncode for VbftBlockInfo {
    fn dep_encode<O: NestedEncodeOutput>(&self, dest: &mut O) -> Result<(), EncodeError> {
        let mut sink = ZeroCopySink::new();

        sink.write_u32(self.proposer);
        sink.write_var_bytes(self.vrf_value.as_slice());
        sink.write_var_bytes(self.vrf_proof.as_slice());
        sink.write_u32(self.last_config_block_num);

        if let Some(chain_config) = &self.new_chain_config {
            match chain_config.dep_encode(&mut sink) {
                Ok(()) => {}
                Err(err) => return Err(err),
            };
        }

        dest.write(sink.get_sink().as_slice());

        Ok(())
    }
}

impl NestedDecode for VbftBlockInfo {
    fn dep_decode<I: NestedDecodeInput>(input: &mut I) -> Result<Self, DecodeError> {
        let mut source = ZeroCopySource::new(input.flush());

        Self::decode_from_source(&mut source, false)
    }
}

impl TopEncode for VbftBlockInfo {
    #[inline]
    fn top_encode<O: TopEncodeOutput>(&self, output: O) -> Result<(), EncodeError> {
        top_encode_from_nested(self, output)
    }
}

impl TopDecode for VbftBlockInfo {
    fn top_decode<I: TopDecodeInput>(input: I) -> Result<Self, DecodeError> {
        top_decode_from_nested(input)
    }
}
