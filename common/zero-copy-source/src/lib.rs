#![no_std]

use elrond_wasm::api::BigUintApi;
use elrond_wasm::elrond_codec::*;
use elrond_wasm::types::{Address, BoxedBytes, Vec, H256};

#[derive(Clone)]
pub struct ZeroCopySource {
    source: Vec<u8>,
    index: usize,
}

impl NestedDecodeInput for ZeroCopySource {
    fn remaining_len(&mut self) -> usize {
        self.get_bytes_left()
    }

    fn read_into(&mut self, into: &mut [u8]) -> Result<(), DecodeError> {
        if self.get_bytes_left() >= into.len() {
            for i in 0..into.len() {
                into[i] = self.source[self.index + i];
            }

            return Ok(());
        }

        return Err(DecodeError::INPUT_TOO_SHORT);
    }

    fn read_into_or_exit<ExitCtx: Clone>(
        &mut self,
        into: &mut [u8],
        c: ExitCtx,
        exit: fn(ExitCtx, DecodeError) -> !,
    ) {
        match self.read_into(into) {
            Ok(()) => (),
            Err(err) => exit(c, err),
        }
    }

    fn read_slice(&mut self, length: usize) -> Result<&[u8], DecodeError> {
        if self.get_bytes_left() >= length {
            let slice = &self.source[self.index..(self.index + length)];
            self.index += length;

            Ok(slice)
        } else {
            Err(DecodeError::INPUT_TOO_SHORT)
        }
    }

    fn read_slice_or_exit<ExitCtx: Clone>(
        &mut self,
        length: usize,
        c: ExitCtx,
        exit: fn(ExitCtx, DecodeError) -> !,
    ) -> &[u8] {
        let result = self.read_slice(length);

        if result.is_ok() {
            result.unwrap_or_default()
        } else {
            exit(c, result.unwrap_err());
        }
    }

    fn flush(&mut self) -> &[u8] {
        let src = &self.source[self.index..];

        self.index = self.source.len();

        src
    }
}

// little endian encoding is used
impl ZeroCopySource {
    pub fn new(source: &[u8]) -> Self {
        let mut src = Vec::new();
        src.extend_from_slice(source);

        ZeroCopySource {
            source: src,
            index: 0,
        }
    }

    pub fn get_source(&self) -> BoxedBytes {
        BoxedBytes::from(self.source.as_slice())
    }

    pub fn get_bytes_left(&self) -> usize {
        self.source.len() - self.index
    }

    pub fn next_bytes(&mut self, len: usize) -> Option<BoxedBytes> {
        if self.get_bytes_left() >= len {
            let boxed = BoxedBytes::from(&self.source[self.index..(self.index + len)]);
            self.index += len;

            Some(boxed)
        } else {
            None
        }
    }

    pub fn next_u8(&mut self) -> Option<u8> {
        let size_u8 = core::mem::size_of::<u8>();
        if self.get_bytes_left() >= size_u8 {
            let val = self.source[self.index];
            self.index += size_u8;

            Some(val)
        } else {
            None
        }
    }

    pub fn next_bool(&mut self) -> Option<bool> {
        match self.next_u8() {
            Some(val) => {
                if val == 1 {
                    Some(true)
                } else if val == 0 {
                    Some(false)
                } else {
                    None
                }
            }
            None => None,
        }
    }

    pub fn next_u16(&mut self) -> Option<u16> {
        if self.get_bytes_left() >= core::mem::size_of::<u16>() {
            let b0 = self.next_u8().unwrap_or_default() as u16;
            let b1 = self.next_u8().unwrap_or_default() as u16;

            Some((b1 << 8) | b0)
        } else {
            None
        }
    }

    pub fn next_u32(&mut self) -> Option<u32> {
        if self.get_bytes_left() >= core::mem::size_of::<u32>() {
            let b10 = self.next_u16().unwrap_or_default() as u32;
            let b32 = self.next_u16().unwrap_or_default() as u32;

            Some((b32 << 16) | b10)
        } else {
            None
        }
    }

    pub fn next_u64(&mut self) -> Option<u64> {
        if self.get_bytes_left() >= core::mem::size_of::<u64>() {
            let b3210 = self.next_u32().unwrap_or_default() as u64;
            let b7654 = self.next_u32().unwrap_or_default() as u64;

            Some((b7654 << 32) | b3210)
        } else {
            None
        }
    }

    /// Gets the next u256, but converts it to BigUint (which also cuts the leading zeroes)
    pub fn next_u256<BigUint: BigUintApi>(&mut self) -> Option<BigUint> {
        let nr_bytes = H256::len_bytes();

        match self.next_bytes(nr_bytes) {
            Some(bytes) => {
                let mut vec = Vec::with_capacity(nr_bytes);
                vec.extend_from_slice(bytes.as_slice());
                vec.reverse();

                Some(BigUint::from_bytes_be(&vec))
            }
            None => None,
        }
    }

    pub fn next_var_uint(&mut self) -> Option<u64> {
        let opt_len_id = self.next_u8();

        match opt_len_id {
            Some(len_id) => match len_id {
                0xfd => self.next_u16().map(|val| val as u64),
                0xfe => self.next_u32().map(|val| val as u64),
                0xff => self.next_u64(),
                _ => Some(len_id as u64),
            },
            None => None,
        }
    }

    pub fn next_var_bytes(&mut self) -> Option<BoxedBytes> {
        let opt_len = self.next_var_uint();

        match opt_len {
            Some(len) => self.next_bytes(len as usize),
            None => None,
        }
    }

    pub fn next_address(&mut self) -> Option<Address> {
        match self.next_bytes(Address::len_bytes()) {
            Some(address_bytes) => Some(Address::from_slice(address_bytes.as_slice())),
            None => None,
        }
    }

    pub fn next_hash(&mut self) -> Option<H256> {
        match self.next_bytes(H256::len_bytes()) {
            Some(hash_bytes) => Some(H256::from_slice(hash_bytes.as_slice())),
            None => None,
        }
    }
}
