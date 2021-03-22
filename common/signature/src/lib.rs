#![no_std]

use elrond_wasm::elrond_codec::*;
use elrond_wasm::types::Box;

use zero_copy_sink::*;
use zero_copy_source::*;

elrond_wasm::derive_imports!();

const SIGNATURE_LENGTH: usize = 64;

#[derive(TypeAbi, Debug, Clone, Copy, PartialEq)]
pub enum SignatureScheme {
    SHA224withECDSA,
	SHA256withECDSA,
	SHA384withECDSA,
	SHA512withECDSA,
	SHA3_224withECDSA,
	SHA3_256withECDSA,
	SHA3_384withECDSA,
	SHA3_512withECDSA,
	RIPEMD160withECDSA,

	SM3withSM2,

	SHA512withEDDSA,

    Unknown,
}

impl From<u8> for SignatureScheme {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::SHA224withECDSA,
            1 => Self::SHA256withECDSA,
            2 => Self::SHA384withECDSA,
            3 => Self::SHA512withECDSA,
            4 => Self::SHA3_224withECDSA,
            5 => Self::SHA3_256withECDSA,
            6 => Self::SHA3_384withECDSA,
            7 => Self::SHA3_512withECDSA,
            8 => Self::RIPEMD160withECDSA,
            9 => Self::SM3withSM2,
            10 => Self::SHA512withEDDSA,
            _ => Self::Unknown
        }
    }
}

#[derive(TypeAbi, Debug, PartialEq)]
pub struct Signature {
    pub scheme: SignatureScheme,
    pub value: Box<[u8; SIGNATURE_LENGTH]>
}

impl Signature {
    pub fn value_as_slice(&self) -> &[u8] {
        &(*self.value)[..]
    }

    pub fn decode_from_source(source: &mut ZeroCopySource) -> Result<Self, DecodeError> {
        let scheme;
        let value ;

        match source.next_u8() {
			Some(val) => {
                let enum_value = SignatureScheme::from(val);
                if enum_value != SignatureScheme::Unknown {
                    scheme = enum_value;
                }
                else {
                    return Err(DecodeError::INVALID_VALUE)
                }
            }
			None => return Err(DecodeError::INPUT_TOO_SHORT)
		};

        match source.next_bytes(SIGNATURE_LENGTH) {
            Some(sig) => {
                let mut sig_array = [0u8; SIGNATURE_LENGTH];
                sig_array.copy_from_slice(sig.as_slice());

                value = Box::from(sig_array);
            },
            None => return Err(DecodeError::INPUT_TOO_SHORT)
        }

        Ok(Self {
            scheme,
            value
        })
    }
}

impl NestedEncode for Signature {
    fn dep_encode<O: NestedEncodeOutput>(&self, dest: &mut O) -> Result<(), EncodeError> {
        let mut sink = ZeroCopySink::new();

        sink.write_u8(self.scheme as u8);
        sink.write_bytes(self.value_as_slice());

        dest.write(sink.get_sink().as_slice());

        Ok(())
    }
}

impl NestedDecode for Signature {
    fn dep_decode<I: NestedDecodeInput>(input: &mut I) -> Result<Self, DecodeError> {
        let mut source = ZeroCopySource::new(input.flush());

        Self::decode_from_source(&mut source)
    }
}

impl TopEncode for Signature {
    #[inline]
    fn top_encode<O: TopEncodeOutput>(&self, output: O) -> Result<(), EncodeError> {
        top_encode_from_nested(self, output)
    }
}

impl TopDecode for Signature {
    fn top_decode<I: TopDecodeInput>(input: I) -> Result<Self, DecodeError> {
        top_decode_from_nested(input)
    }
}
