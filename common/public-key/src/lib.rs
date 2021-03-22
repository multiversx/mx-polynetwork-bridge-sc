#![no_std]

use elrond_wasm::elrond_codec::*;
use elrond_wasm::types::BoxedBytes;

use zero_copy_sink::*;
use zero_copy_source::*;

elrond_wasm::derive_imports!();

#[derive(TypeAbi, Debug, Clone, Copy, PartialEq)]
pub enum EllipticCurveAlgorithm {
    ECDSA,
    SM2,

    Unknown,
}

impl From<u8> for EllipticCurveAlgorithm {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::ECDSA,
            1 => Self::SM2,
            _ => Self::Unknown
        }
    }
}

#[derive(TypeAbi, Debug, PartialEq)]
pub struct PublicKey {
    pub algorithm: EllipticCurveAlgorithm,
    pub value: BoxedBytes
}

impl PublicKey {
    pub fn value_as_slice(&self) -> &[u8] {
        self.value.as_slice()
    }

    pub fn decode_from_source(source: &mut ZeroCopySource) -> Result<Self, DecodeError> {
        let algorithm;
        let value ;

        match source.next_u8() {
			Some(val) => {
                let enum_value = EllipticCurveAlgorithm::from(val);
                if enum_value != EllipticCurveAlgorithm::Unknown {
                    algorithm = enum_value;
                }
                else {
                    return Err(DecodeError::INVALID_VALUE)
                }
            }
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

        match source.next_var_bytes() {
            Some(sig) => {
                value = sig;
            },
            None => return Err(DecodeError::INPUT_TOO_SHORT)
        }

        Ok(Self {
            algorithm,
            value
        })
    }
}

impl NestedEncode for PublicKey {
    fn dep_encode<O: NestedEncodeOutput>(&self, dest: &mut O) -> Result<(), EncodeError> {
        let mut sink = ZeroCopySink::new();

        sink.write_u8(self.algorithm as u8);
        sink.write_var_bytes(self.value_as_slice());

        dest.write(sink.get_sink().as_slice());

        Ok(())
    }
}

impl NestedDecode for PublicKey {
    fn dep_decode<I: NestedDecodeInput>(input: &mut I) -> Result<Self, DecodeError> {
        let mut source = ZeroCopySource::new(input.flush());

        Self::decode_from_source(&mut source)
    }
}

impl TopEncode for PublicKey {
    #[inline]
    fn top_encode<O: TopEncodeOutput>(&self, output: O) -> Result<(), EncodeError> {
        top_encode_from_nested(self, output)
    }
}

impl TopDecode for PublicKey {
    fn top_decode<I: TopDecodeInput>(input: I) -> Result<Self, DecodeError> {
        top_decode_from_nested(input)
    }
}
