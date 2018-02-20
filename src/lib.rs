//! Compile and deploy Solidity contracts and libraries
extern crate ethabi;
extern crate rustc_hex;
extern crate serde_json;
extern crate web3;

use std::time::Duration;

use web3::api::{Eth, Namespace};
use web3::confirm::*;
use web3::contract::Options;
use web3::Error as Web3Error;
use web3::futures::Future;
use web3::futures::future::{ok, FutureResult};
use web3::Transport;
use web3::types::*;

// public interface
pub mod compile;
pub mod registry;

// TODO: load information from Truffle JSON artifacts

#[derive(Debug)]
struct Library {
    bytecode: Vec<u8>,
    address: Address,
    // eth: Eth<T>
}

impl Library {
    pub fn deploy<T: Transport>(
        eth: Eth<T>,
        bytecode: Vec<u8>,
        from: Address,
        options: Options,
    ) -> FutureResult<Library, Web3Error> {
        let poll_interval = Duration::from_secs(1);
        let confirmations = 0;

        let transaction_request = TransactionRequest {
            from: from,
            to: None,
            gas: options.gas,
            gas_price: options.gas_price,
            value: options.value,
            nonce: options.nonce,
            data: Some(Bytes(bytecode.clone())),
            condition: options.condition,
        };

        let tx = send_transaction_with_confirmation(
            eth.transport().clone(),
            transaction_request,
            poll_interval,
            confirmations,
        );
        let receipt = tx.wait().unwrap();
        // println!("tx: {:#?}", receipt);

        ok(Library {
            bytecode,
            address: receipt.contract_address.unwrap(),
        })
    }

    pub fn address(&self) -> Address {
        self.address
    }
}

// fn call_with_confirmations<'a, P, T>(
//     transport: &'a T,
//     contract: Address,
//     abi: &ethabi::Contract,
//     func: &str,
//     params: P,
//     from: Address,
//     confirmations: usize,
// ) -> web3::confirm::SendTransactionWithConfirmation<&'a T>
// where
//     P: Tokenize,
//     T: 'a + Transport,
// {
//     let options = Options::default();
//     let poll_interval = Duration::from_secs(1);

//     let fn_data = abi.function(func.into())
//         .and_then(|function| function.encode_input(&params.into_tokens()))
//         .unwrap();

//     let transaction_request = TransactionRequest {
//         from: from,
//         to: Some(contract.clone()),
//         gas: options.gas,
//         gas_price: options.gas_price,
//         value: options.value,
//         nonce: options.nonce,
//         data: Some(Bytes(fn_data)),
//         condition: options.condition,
//     };

//     send_transaction_with_confirmation(transport, transaction_request, poll_interval, confirmations)
// }

#[cfg(test)]
mod test {
    // use super::*;

}
