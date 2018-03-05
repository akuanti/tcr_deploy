# TCR Deployer

This is a script for compiling and deploying a [token-curated registry](https://github.com/skmgoldin/tcr). It is intended for **testing purposes only!**

To run, adjust the params in `conf/config.json`, then run the main script with cargo:
```
cargo run
```

You can also import it as a library and pass in the path to your configuration file to `deploy()`.

## Requirements
You must have a Solidity compiler installed and on your `PATH`. You must also have an Ethereum node to connect to.

## Configuration
Specify the parameters for the TCR's parameterizer, and compile/deployment options in a file.

All fields are required except `gasLimit` and `listingDeposit`.

* `params` - Parameterizer constructor arguments
* `tcrDirectory` - the directory with the TCR code
* `compilerBuildDirectory` - the directory to store the compiled files
* `gasLimit` - OPTIONAL: gas limit to use for deployments
* `listingDeposit` - OPTIONAL: deposit to use for adding listings
* `listings` - listings to add

```json
{
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
  "compilerBuildDirectory": "test_build",
  "gasLimit": 3000000,
  "listingDeposit": 20,
  "listings": [
      "abc.com",
      "def.com"
  ]
}
```

## Example Usage

```rust
extern crate tcr_deploy;
extern crate web3;

use std::error::Error;
use tcr_deploy::registry;

fn main() {
    // initialize web3
    let (_eloop, http) = web3::transports::Http::new("http://localhost:8545").unwrap();
    let web3 = web3::Web3::new(http);

    // load config file
    match tcr_deploy::Config::load_json("conf/config.json") {
        Ok(config) => {
            // build and deploy contracts
            let mut registry_info = registry::deploy(&web3, &config);
            let registry_contract = &registry_info.registry;
            println!("REGISTRY {:?}", registry_contract.address());

            // add listings
            let deposit = config.deposit();
            for listing in config.listings() {
                registry::add_listing(&web3, &registry_info, listing, deposit);
            }
        }
        Err(e) => println!("Problem loading config {:?}", e.description()),
    }
}
```
