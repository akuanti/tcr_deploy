//! Compile and deploy Solidity contracts and libraries
extern crate ethabi;
extern crate rustc_hex;
extern crate serde;
extern crate serde_json;
extern crate web3;

#[macro_use]
extern crate serde_derive;

use std::fs::File;
use std::io;
use std::io::Read;
use std::time::Duration;

use web3::api::{Eth, Namespace};
use web3::confirm::*;
use web3::contract::Options;
use web3::Error as Web3Error;
use web3::futures::Future;
use web3::futures::future::{ok, FutureResult};
use web3::Transport;
use web3::types::*;

use registry::Parameters;

// public interface
pub mod registry;

// TODO: load information from Truffle JSON artifacts

#[derive(Debug, Deserialize)]
/// TCR deployment and compiler configuration
pub struct Config {
    /// Parameters for TCR constructor
    params: Parameters,
    /// The directory with the TCR code
    #[serde(rename = "tcrDirectory")]
    tcr_dir: String,
    /// The directory to put the compiler output, relative to `tcr_dir`
    /// Default: "build"
    #[serde(rename = "compilerBuildDirectory")]
    compiler_build_dir: Option<String>,
}

impl Config {
    /// Create a new `Config`
    pub fn new(params: Parameters, tcr_dir: &str, compiler_build_dir: Option<&str>) -> Config {
        Config {
            params,
            tcr_dir: tcr_dir.to_string(),
            compiler_build_dir: match compiler_build_dir {
                Some(s) => Some(s.to_string()),
                None => None,
            },
        }
    }

    /// Create a `Config` from  a json file
    pub fn load_json(path: &str) -> Result<Config, io::Error> {
        // load json string from file
        let mut f = File::open(path)?;

        let mut json = String::new();
        f.read_to_string(&mut json)?;

        // parse json
        serde_json::from_str(json.as_str())
            .map_err(|_e: serde_json::Error| io::Error::new(io::ErrorKind::Other, _e))
    }

    /// Set the output directory for the compiler
    pub fn compiler_build_dir(&self) -> &str {
        match self.compiler_build_dir {
            None => "build",
            Some(ref s) => s.as_str(),
        }
    }

    /// Set the directory where the TCR code is stored
    pub fn tcr_dir(&self) -> &str {
        self.tcr_dir.as_str()
    }
}

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
    use super::*;

    const EXAMPLE_PARAMS: &'static str = r#"{
        "minDeposit": 10,
        "pMinDeposit": 100,
        "applyStageLength": 0,
        "pApplyStageLength": 1200,
        "commitStageLength": 600,
        "pCommitStageLength": 1200,
        "revealStageLength": 600,
        "pRevealStageLength": 1200,
        "dispensationPct": 50,
        "pDispensationPct": 50,
        "voteQuorum": 50,
        "pVoteQuorum": 50
    }"#;

    #[test]
    fn should_deserialize_config() {
        let data = r#"{
            "params": {
                "minDeposit": 10,
                "pMinDeposit": 100,
                "applyStageLength": 0,
                "pApplyStageLength": 1200,
                "commitStageLength": 600,
                "pCommitStageLength": 1200,
                "revealStageLength": 600,
                "pRevealStageLength": 1200,
                "dispensationPct": 50,
                "pDispensationPct": 50,
                "voteQuorum": 50,
                "pVoteQuorum": 50
            },
            "tcrDirectory": "../tcr",
            "solcOutputDir": "some_place"
        }"#;

        let config: Config = serde_json::from_str(data).expect("Could not parse config");
    }

    #[test]
    fn should_construct_config() {
        let params = serde_json::from_str(EXAMPLE_PARAMS).expect("Could not parse params");
        let config = Config::new(params, "../tcr", Some("../tcr/output"));

        assert_eq!(config.tcr_dir(), "../tcr");
    }

    #[test]
    fn should_construct_config_with_default_output_dir() {
        let params = serde_json::from_str(EXAMPLE_PARAMS).expect("Could not parse params");
        let config = Config::new(params, "../tcr", None);
    }

    #[test]
    #[should_panic]
    fn should_fail_with_missing_file() {
        let config = Config::load_json("non-existent-file.json").expect("Config file not found");
    }
}
