extern crate web3;
extern crate rustc_hex;
extern crate serde_json;

use web3::futures::Future;
use web3::contract::{Contract, Options};
use web3::types::{Address, U256, BlockNumber};
use rustc_hex::FromHex;
use std::process::Command;

//deploy token
//deploy dll
//deploy atrstore
//link/deploy PLCRVoting
//link/deploy Parameterizer
//link/deploy Registry

fn main() {
    let (_eloop, http) = web3::transports::Http::new("http://localhost:8545").unwrap();
    let web3 = web3::Web3::new(http);

    let my_account: Address = "0x00a329c0648769a73afac7f9381e08fb43dbea72".parse().unwrap();
    // Get the contract bytecode for instance from Solidity compiler
    let eip20_bytecode: Vec<u8> = include_str!("../build/EIP20.bin").from_hex().unwrap();
    let dll_bytecode: Vec<u8> = include_str!("../build/DLL.bin").from_hex().unwrap();
    let attribute_store_bytecode: Vec<u8> = include_str!("../build/AttributeStore.bin").from_hex().unwrap();

    let eip20_contract = Contract::deploy(web3.eth(), include_bytes!("../build/EIP20.abi")).unwrap()
      .confirmations(0)
      .options(Options::with(|mut opt| opt.gas = Some(5_000_000.into())))
      .execute(eip20_bytecode, (
      	U256::from(1_000_000),
      	"TestCoin".to_owned(),
      	U256::from(0),
      	"TEST".to_owned()
      	), my_account)
      .expect("Correct parameters are passed to the constructor.")
      .wait()
      .unwrap();

    println!("{:?}", eip20_contract.address());

    let val = [0;0];

    let dll_contract = Contract::deploy(web3.eth(), include_bytes!("../build/DLL.abi")).unwrap()
      .confirmations(0)
      .options(Options::with(|mut opt| opt.gas = Some(5_000_000.into())))
      .execute(dll_bytecode, (), my_account)
      .expect("Correct parameters are passed to the constructor.")
      .wait()
      .unwrap();

    println!("{:?}", dll_contract.address());

    let attribute_store_contract = Contract::deploy(web3.eth(), include_bytes!("../build/AttributeStore.abi")).unwrap()
      .confirmations(0)
      .options(Options::with(|mut opt| opt.gas = Some(5_000_000.into())))
      .execute(attribute_store_bytecode, (), my_account)
      .expect("Correct parameters are passed to the constructor.")
      .wait()
      .unwrap();

    println!("{:?}", attribute_store_contract.address());


}
