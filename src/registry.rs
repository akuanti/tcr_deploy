//! TCR functions
extern crate solc;
extern crate tiny_keccak;
extern crate web3;

use std::env;
use std::path::PathBuf;

use web3::futures::Future;
use web3::futures::future::ok;
use web3::contract::{Contract, Options};
use web3::types::{Address, H256, U256};
use web3::Transport;

use Library;
use Config;

/// TCR parameters
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parameters {
    pub(crate) min_deposit: u32,
    apply_stage_length: u32,
    commit_stage_length: u32,
    reveal_stage_length: u32,
    #[serde(rename = "dispensationPct")]
    dispensation_percentage: u32,
    vote_quorum: u32,
    // parameterizer
    p_min_deposit: u32,
    p_apply_stage_length: u32,
    p_commit_stage_length: u32,
    p_reveal_stage_length: u32,
    #[serde(rename = "pDispensationPct")]
    p_dispensation_percentage: u32,
    p_vote_quorum: u32,
}

/// TCR info
#[derive(Debug)]
pub struct RegistryInfo<T>
where
    T: Transport,
{
    pub registry: Contract<T>,
    pub token: Contract<T>,
    pub voting: Contract<T>,
}

// TODO: return Result
fn compile_libraries(compiler: &solc::Solc) {
    println!("compiling in {}", compiler.root());
    println!("output dir: {}", compiler.output_dir());

    let mut abs_path: PathBuf = env::current_dir().unwrap();
    abs_path.extend(&["../tcr", "installed_contracts"]);
    let abs_path: PathBuf = abs_path.canonicalize().unwrap();
    let abs_path: &str = abs_path.to_str().unwrap();

    let mut compile = compiler.compile();
    compile
        // binaries only for libraries
        .bin()
        .allow_path(abs_path)
        .add_source("installed_contracts/dll/contracts/DLL.sol")
        .add_source("installed_contracts/attrstore/contracts/AttributeStore.sol")
        .overwrite();

    let cmd = compile.execute().expect("No command");
    let output = cmd.output().expect("Failed to compile libs");

    println!("command {:?}", output);
    println!("{}", String::from_utf8_lossy(&output.stderr));
}

// TODO: return Result
/// Deploy DLL and AttributeStore and save the library addresses
fn deploy_libraries<T>(t: &web3::Web3<T>, my_account: Address, compiler: &mut solc::Solc)
where
    T: Transport,
{
    // TODO: run the library deployments in parallel

    let dll_bytecode: Vec<u8> = compiler.load_bytecode("DLL.bin");
    let attribute_store_bytecode: Vec<u8> = compiler.load_bytecode("AttributeStore.bin");

    println!("deploying DLL");
    let library_deploy = Library::deploy(
        t.eth(),
        dll_bytecode,
        my_account.clone(),
        Options::with(|opt| opt.gas = Some(1_000_000.into())),
    ).and_then(|dll| {
        compiler.add_library_address("DLL", dll.address());

        println!("deploying AttributeStore");
        let attr_deploy = Library::deploy(
            t.eth(),
            attribute_store_bytecode,
            my_account.clone(),
            Options::with(|opt| opt.gas = Some(1_000_000.into())),
        );
        let deployed = attr_deploy.wait();

        ok((deployed.unwrap(), compiler))
    })
    .and_then(|(attr, compiler)|{
        compiler.add_library_address("AttributeStore", attr.address());
        compiler.prepare_link();
        ok(())
    });

    library_deploy.wait().unwrap();
}

// compile and link dependent contracts
// PLCRVoting -> EIP20, [DLL], [AttributeStore]
// Parameterizer -> PLCRVoting, EIP20
// Registry -> EIP20, Parameterizer, PLCRVoting
fn compile_contracts(compiler: &solc::Solc) -> Result<(), &'static str> {
    println!("compiling contracts");

    let epm_dir = "installed_contracts";
    let mut epm_dir_abs = env::current_dir().expect("Could not get current directory");
    epm_dir_abs.extend(&[compiler.root(), epm_dir]);
    let epm_dir_abs = epm_dir_abs
        .canonicalize()
        .expect("Could not canonicalize path");

    let mut compile = compiler.compile();
    compile
        .bin()
        .abi()
        .overwrite()
        .link()
        .allow_path(epm_dir_abs.to_str().unwrap());

    // remap paths for EPM libs
    for p in ["dll", "attrstore", "tokens"].iter() {
        let _path = format!("{}/{}/contracts", epm_dir, p);
        compile.add_mapping(p, _path.as_str());
    }

    // input Solidity files
    for s in ["PLCRVoting.sol", "Parameterizer.sol", "Registry.sol"].iter() {
        let src = format!("contracts/{}", s);
        compile.add_source(src.as_str());
    }

    let cmd = compile.execute().expect("No command found");
    println!("{:?}", cmd);

    let output = cmd.output().expect("failed to execute process");

    match output.status.code() {
        Some(0) => Ok(()),
        Some(1) => {
            let _stderr = String::from_utf8_lossy(&output.stderr);
            // if "Refusing to overwrite" in stderr, count as ok
            if _stderr.contains("Refusing to overwrite") {
                Ok(())
            } else {
                // println!("{}", _stderr);
                Err("Compiler error")
            }
        }
        _ => Err("something went wrong"),
    }
}

/// Run the entire deployment of a TCR
// deploy dll
// deploy token
// deploy attrstore
// link/deploy PLCRVoting
// link/deploy Parameterizer
// link/deploy Registry
//
// TODO: check network id?
pub fn deploy<T>(web3: &web3::Web3<T>, config: &Config) -> RegistryInfo<T>
where
    T: Transport,
{
    let my_account: Address = web3.eth().accounts().wait().unwrap()[0];

    // contract root = HERE
    let contract_root = env::current_dir().unwrap();
    let contract_root = contract_root.to_str().unwrap();
    println!("ROOT: {:?}", contract_root);

    // relative to tcr root
    let build_dir = config.compiler_build_dir();

    let mut compiler = solc::Solc::new(config.tcr_dir());
    // output_dir is relative to root
    compiler.output_dir = Some(build_dir);

    // TODO: stop if this fails
    compile_libraries(&compiler);

    // deploy libraries and save their addresses in a text file
    // let lib_file: PathBuf = ["libs.txt"].iter().collect();
    deploy_libraries(&web3, my_account, &mut compiler);

    // check if the libraries have been deployed
    // check if the libraries file exists
    // if it does, load the values
    //   check if each lib is on the blockchain
    //   if not, deploy it
    // if not, deploy all

    // compile and link contracts
    // TODO: only if the libraries have been deployed
    compile_contracts(&compiler).expect("Problem compiling contracts");

    // deploy contracts (in order)
    // EIP20
    // PLCRVoting
    // Parameterizer
    // Registry
    println!("deploying contracts");

    let max_gas: U256 = config.gas_limit.unwrap_or(4_500_000).into();

    // Token
    let eip20_bytecode: Vec<u8> = compiler.load_bytecode("EIP20.bin");
    let eip20_contract = Contract::deploy(web3.eth(), &compiler.load_abi("EIP20.abi"))
        .unwrap()
        .confirmations(0)
        .options(Options::with(|opt| opt.gas = Some(max_gas)))
        .execute(
            eip20_bytecode,
            (
                U256::from(1_000_000),
                "TestCoin".to_owned(),
                U256::from(0),
                "TEST".to_owned(),
            ),
            my_account,
        ) // Result<PendingContract>
        .expect("Correct parameters are passed to the constructor.")
        .wait()
        .unwrap();

    println!("EIP20:{:?}", eip20_contract.address());

    // PLCR Voting
    let plcr_bytecode: Vec<u8> = compiler.load_bytecode("PLCRVoting.bin");
    let plcr_contract = Contract::deploy(web3.eth(), &compiler.load_abi("PLCRVoting.abi"))
        .unwrap()
        .confirmations(0)
        .options(Options::with(|opt| opt.gas = Some(max_gas)))
        .execute(plcr_bytecode, (eip20_contract.address(),), my_account)
        .expect("Correct parameters are passed to the constructor.")
        .wait()
        .unwrap();

    println!("PLCRVoting:{:?}", plcr_contract.address());

    // Paraeterizer
    let parameterizer_bytecode: Vec<u8> = compiler.load_bytecode("Parameterizer.bin");
    let params = &config.params;

    let parameterizer_contract =
        Contract::deploy(web3.eth(), &compiler.load_abi("Parameterizer.abi"))
            .unwrap()
            .confirmations(0)
            .options(Options::with(|opt| opt.gas = Some(max_gas)))
            .execute(
                parameterizer_bytecode,
                (
                    eip20_contract.address(),
                    plcr_contract.address(),
                    U256::from(params.min_deposit),
                    U256::from(params.p_min_deposit),
                    U256::from(params.apply_stage_length),
                    U256::from(params.p_apply_stage_length),
                    U256::from(params.commit_stage_length),
                    U256::from(params.p_commit_stage_length),
                    U256::from(params.reveal_stage_length),
                    U256::from(params.p_reveal_stage_length),
                    U256::from(params.dispensation_percentage),
                    U256::from(params.p_dispensation_percentage),
                    U256::from(params.vote_quorum),
                    U256::from(params.p_vote_quorum),
                ),
                my_account,
            )
            .expect("Correct parameters to be passed into constructor")
            .wait()
            .expect("Problem deploying parameterizer");

    println!("Parameterizer:{:?}", parameterizer_contract.address());

    // Registry
    let registry_bytecode: Vec<u8> = compiler.load_bytecode("Registry.bin");
    let pending = Contract::deploy(web3.eth(), &compiler.load_abi("Registry.abi"))
        .unwrap()
        .confirmations(0)
        .options(Options::with(|opt| opt.gas = Some(max_gas)))
        .execute(
            registry_bytecode,
            (
                eip20_contract.address(),
                plcr_contract.address(),
                parameterizer_contract.address(),
                String::from("awesome test registry"),
            ),
            my_account,
        )
        .expect("Correct parameters to be passed into constructor");

    let registry_contract = pending.wait().expect("Problem with registry deployment");

    // execute() -> Result<PendingContract>
    // PendingContract is a future
    //
    println!("Registry:{:?}", registry_contract.address());

    RegistryInfo {
        registry: registry_contract,
        token: eip20_contract,
        voting: plcr_contract,
    }
}

/// Makes application with a given deposit
pub fn add_listing<T>(web3: &web3::Web3<T>, info: &RegistryInfo<T>, name: &str, deposit: u32)
where
    T: Transport,
{
    println!("Adding listing {}", name);

    // set up
    let accounts = web3.eth()
        .accounts()
        .wait()
        .expect("Could not get accounts");

    let deposit = U256::from(deposit);
    let confirmations = 0;

    // approve registry to spend deposit
    let token_contract = &info.token;
    println!("Approving registry to spend {}", deposit);

    let result = token_contract.call_with_confirmations(
        "approve",
        (info.registry.address(), deposit),
        accounts[0],
        Options::default(),
        confirmations,
    );
    let receipt = result.wait().expect("Could not approve funds");

    println!("receipt: {:?}", receipt);

    // apply with domain
    // TODO: get waiting period from contract
    let mut sha3 = tiny_keccak::Keccak::new_sha3_256();
    let data: Vec<u8> = From::from(name);
    sha3.update(&data);

    let mut res: [u8; 32] = [0; 32];
    sha3.finalize(&mut res);

    let result = info.registry.call_with_confirmations(
        "apply",
        (H256::from(res), deposit, name.to_owned()),
        accounts[0],
        Options::with(|opt| opt.gas = Some(1_000_000.into())),
        confirmations,
    );
    let receipt = result.wait().expect("Could not submit application");
    println!("receipt: {:?}", receipt);

    // wait for apply period

    // update status
    let result = info.registry.call_with_confirmations(
        "updateStatus",
        (H256::from(res),),
        accounts[0],
        Options::default(),
        confirmations,
    );
    let status = result.wait().expect("Could not update status");
    println!("updateStatus: {:?}", status);

    let result = info.registry.query(
        "isWhitelisted",
        H256::from(res),
        None,
        Options::default(),
        None,
    );
    let added: bool = result.wait().expect("Problem checking whitelist");
    println!("isWhitelisted: {:?}", added);
}
