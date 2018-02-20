extern crate tcr_deploy;
extern crate web3;

use tcr_deploy::registry;

fn main() {
    // deploy_tcr --params <file> --tcr <dir>
    let (_eloop, http) = web3::transports::Http::new("http://localhost:8545").unwrap();
    let web3 = web3::Web3::new(http);

    // TODO: optionally pass in existing registry contract address
    // deploy_or_get()
    let mut registry_info = registry::deploy(&web3);
    let registry_contract = &registry_info.registry;
    println!("REGISTRY {:?}", registry_contract.address());


}
