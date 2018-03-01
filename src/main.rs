extern crate tcr_deploy;
extern crate web3;

use std::error::Error;
use tcr_deploy::registry;

fn main() {
    // deploy_tcr --params <file> --tcr <dir>
    let (_eloop, http) = web3::transports::Http::new("http://localhost:8545").unwrap();
    let web3 = web3::Web3::new(http);

    // load config file
    match tcr_deploy::Config::load_json("conf/config.json") {
        Ok(config) => {
            println!("CONFIG: {:#?}", config);

            // TODO: optionally pass in existing registry contract address
            let mut registry_info = registry::deploy(&web3, config);
            let registry_contract = &registry_info.registry;
            println!("REGISTRY {:?}", registry_contract.address());

            println!("Adding listings");
            registry::add_listing(&web3, &registry_info, "abc.com");
        }
        Err(e) => println!("Problem loading config {:?}", e.description()),
    }
}
