
#![no_std]

imports!();

#[elrond_wasm_derive::contract(BlockHeaderSyncImpl)]
pub trait BlockHeaderSync {
    #[init]
    fn init(&self) {
        
    }

    // utils

    /*fn hash_raw_header(&self, header: &Header) -> Option<H256> {
        let mut bytes = Vec::<u8>::new();
        match header.dep_encode(&mut bytes) {
            core::result::Result::Ok(()) => Some(self.sha256(self.sha256(bytes.as_slice()).as_bytes())),
            core::result::Result::Err(_en_err) => None
        }
    }*/

    /*fn get_next_book_keeper(&self, pub_key_list: Vec<PublicKey>) -> (BoxArray20, Vec<Address>) {
        
    }*/
}
