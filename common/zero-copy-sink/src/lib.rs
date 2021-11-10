#![no_std]

use elrond_wasm::api::BigUintApi;
use elrond_wasm::elrond_codec::*;
use elrond_wasm::types::{Address, BoxedBytes, Vec, H256};

pub struct ZeroCopySink {
    sink: Vec<u8>,
}

impl NestedEncodeOutput for ZeroCopySink {
    fn write(&mut self, bytes: &[u8]) {
        self.write_bytes(bytes);
    }
}

impl Default for ZeroCopySink {
    fn default() -> Self {
        Self::new()
    }
}

// little endian encoding is used
impl ZeroCopySink {
    pub fn new() -> Self {
        ZeroCopySink { sink: Vec::new() }
    }

    pub fn get_sink(&self) -> BoxedBytes {
        BoxedBytes::from(self.sink.as_slice())
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.sink.extend_from_slice(bytes);
    }

    pub fn write_u8(&mut self, byte: u8) {
        self.sink.push(byte);
    }

    pub fn write_bool(&mut self, boolean: bool) {
        if boolean {
            self.write_u8(1u8);
        } else {
            self.write_u8(0u8);
        }
    }

    pub fn write_u16(&mut self, val: u16) {
        self.write_u8(val as u8);
        self.write_u8((val >> 8) as u8);
    }

    pub fn write_u32(&mut self, val: u32) {
        self.write_u16(val as u16);
        self.write_u16((val >> 16) as u16)
    }

    pub fn write_u64(&mut self, val: u64) {
        self.write_u32(val as u32);
        self.write_u32((val >> 32) as u32);
    }

    pub fn write_u256<BigUint: BigUintApi>(&mut self, val: &BigUint) -> Result<(), EncodeError> {
        match val.to_bytes_be_pad_right(H256::len_bytes()) {
            Some(mut bytes) => {
                bytes.reverse();

                self.write_bytes(&bytes);

                Ok(())
            }
            None => Err(EncodeError::from(&b"Failed encoding u256"[..])),
        }
    }

    pub fn write_var_uint(&mut self, val: u64) {
        if val < 0xfd {
            self.write_u8(val as u8);
        } else if val <= 0xffff {
            self.write_u8(0xfd);
            self.write_u16(val as u16);
        } else if val <= 0xffffff {
            self.write_u8(0xfe);
            self.write_u32(val as u32);
        } else {
            self.write_u8(0xff);
            self.write_u64(val);
        }
    }

    pub fn write_var_bytes(&mut self, bytes: &[u8]) {
        self.write_var_uint(bytes.len() as u64);
        self.write_bytes(bytes);
    }

    pub fn write_address(&mut self, address: &Address) {
        self.write_bytes(address.as_bytes());
    }

    pub fn write_hash(&mut self, hash: &H256) {
        self.write_bytes(hash.as_bytes());
    }
}
