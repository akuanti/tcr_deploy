//! TCR functions
extern crate solc;
extern crate tiny_keccak;
extern crate web3;

use std::env;
use std::fmt::Debug;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use web3::futures::Future;
use web3::futures::future::ok;
use web3::contract::{Contract, Options};
use web3::types::{Address, U256, H256};
use web3::Transport;

use rustc_hex::FromHex;

use Library;
use Config;

/// TCR parameters
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parameters {
    min_deposit: u32,
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
    where T: Transport
{
    pub registry: Contract<T>,
    pub token: Contract<T>,
    pub voting: Contract<T>,
}

// TODO: return Result
fn compile_libraries(compiler: &solc::Solc) {
    println!("compiling in {}", compiler.root());
    println!("output dir: {}", compiler.output_dir());

    let output_dir = format!("{}/{}", compiler.root(), compiler.output_dir());

    let mut abs_path: PathBuf = env::current_dir().unwrap();
    abs_path.extend(&["../tcr", "installed_contracts"]);
    let abs_path: PathBuf = abs_path.canonicalize().unwrap();
    let abs_path: &str = abs_path.to_str().unwrap();

    let output = compiler.compile()
        // binaries only for libraries
        .bin()
        .allow_path(abs_path)
        .add_source("installed_contracts/dll/contracts/DLL.sol")
        .add_source("installed_contracts/attrstore/contracts/AttributeStore.sol")
        .output_dir(output_dir.as_str())
        .overwrite()
        .execute()
        .expect("No command")
        .output()
        .expect("Failed to compile libs");

    println!("command {:?}", output);
    println!("{}", String::from_utf8_lossy(&output.stderr));
}

// TODO: return Result
fn deploy_libraries<T, P>(t: &web3::Web3<T>, my_account: Address, lib_file_path: P)
where
    T: Transport,
    P: AsRef<Path> + Debug,
{
    // TODO: run the library deployments in parallel
    // TODO: keep track of the libraries
    println!("libs file: {:?}", lib_file_path.as_ref());
    let mut libraries = Vec::<(&str, Library)>::new();
    let mut lib_file = File::create(lib_file_path).expect("Could not create libs file");

    let dll_bytecode: Vec<u8> = include_str!("../build/DLL.bin").from_hex().unwrap();
    let attribute_store_bytecode: Vec<u8> = include_str!("../build/AttributeStore.bin")
        .from_hex()
        .unwrap();

    println!("deploying DLL");
    let library_deploy = Library::deploy(
        t.eth(),
        dll_bytecode,
        my_account.clone(),
        Options::with(|opt| opt.gas = Some(1_000_000.into())),
    ).and_then(|dll| {
        // println!("DLL: {:#?}", dll.address());
        let line: String = format!("DLL:{:?}\n", dll.address());
        print!("{}", &line);
        lib_file
            .write(line.as_bytes())
            .expect("Could not write to library file");
        libraries.push(("DLL", dll));

        println!("deploying AttributeStore");
        // deploy_library(bytecode, my_account)
        ok((
            Library::deploy(
                t.eth(),
                attribute_store_bytecode,
                my_account.clone(),
                Options::with(|opt| opt.gas = Some(1_000_000.into())),
            ).wait()
                .unwrap(),
            libraries,
            lib_file,
        ))
    })
        .and_then(|(attr, mut libraries, mut lib_file)| {
            let line: String = format!("AttributeStore:{:?}\n", attr.address());
            print!("{}", &line);
            lib_file
                .write(line.as_bytes())
                .expect("Could not write to library file");
            libraries.push(("AttributeStore", attr));
            ok(libraries)
        });

    library_deploy.wait().unwrap();
}

// compile and link dependent contracts
// PLCRVoting -> EIP20, [DLL], [AttributeStore]
// Parameterizer -> PLCRVoting, [EIP20]
// Registry -> EIP20, Parameterizer, PLCRVoting
fn compile_contracts<P>(compiler: &solc::Solc, lib_file_path: P) -> Result<(), &'static str>
where
    P: AsRef<Path>,
{
    println!("compiling contracts");

    let epm_dir = "installed_contracts";
    let mut epm_dir_abs = env::current_dir().expect("Could not get current directory");
    epm_dir_abs.extend(&[compiler.root(), epm_dir]);
    let epm_dir_abs = epm_dir_abs
        .canonicalize()
        .expect("Could not canonicalize path");

    // output dir: <tcr>/<output>/
    let output_dir = [
        compiler.root(),
        compiler.output_dir.expect("No output directory set"),
    ].join("/");

    let mut compile = compiler.compile();
    compile
        .bin()
        .abi()
        .libraries_file(lib_file_path.as_ref().to_str().unwrap())
        .allow_path(epm_dir_abs.to_str().unwrap())
        .output_dir(output_dir.as_str());

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
                Err("Something went wrong")
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
// TODO: pass in tcr dir, build dir
// TODO: check network id?
pub fn deploy<T>(web3: &web3::Web3<T>, config: Config) -> RegistryInfo<T>
where
    T: Transport,
{
    let my_account: Address = web3.eth().accounts().wait().unwrap()[0];

    // contract root = HERE
    let contract_root = env::current_dir().unwrap();
    let contract_root = contract_root.to_str().unwrap();
    println!("ROOT: {:?}", contract_root);

    // relative to tcr root
    const TCR_DIR: &str = "../tcr";
    const BUILD_DIR: &str = "some_place";

    let mut compiler = solc::Solc::new(TCR_DIR);
    // output_dir is relative to root
    compiler.output_dir = Some(BUILD_DIR);

    // TODO: stop if this fails
    compile_libraries(&compiler);

    // deploy libraries and save their addresses in a text file
    let lib_file: PathBuf = [compiler.root(), BUILD_DIR, "libs.txt"].iter().collect();
    deploy_libraries(&web3, my_account, &lib_file);

    // check if the libraries have been deployed
    // check if the libraries file exists
    // if it does, load the values
    //   check if each lib is on the blockchain
    //   if not, deploy it
    // if not, deploy all

    // compile and link contracts
    // TODO: only if the libraries have been deployed
    compile_contracts(&compiler, &lib_file).expect("Problem compiling contracts");

    // deploy contracts (in order)
    // EIP20
    // PLCRVoting
    // Parameterizer
    // Registry
    println!("deploying contracts");

    // Token
    let eip20_bytecode: Vec<u8> = compiler.load_bytecode("EIP20.bin");
    let eip20_contract = Contract::deploy(web3.eth(), &compiler.load_abi("EIP20.abi"))
        .unwrap()
        .confirmations(0)
        .options(Options::with(|opt| opt.gas = Some(1_000_000.into())))
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
        .options(Options::with(|opt| opt.gas = Some(3_000_000.into())))
        .execute(plcr_bytecode, (eip20_contract.address(),), my_account)
        .expect("Correct parameters are passed to the constructor.")
        .wait()
        .unwrap();

    println!("PLCRVoting:{:?}", plcr_contract.address());

    // Paraeterizer
    let parameterizer_bytecode: Vec<u8> = compiler.load_bytecode("Parameterizer.bin");
    // TODO: read from file
    let config = Parameters {
        min_deposit: 10,
        apply_stage_length: 0,
        commit_stage_length: 600,
        reveal_stage_length: 600,
        dispensation_percentage: 50,
        vote_quorum: 50,
        p_min_deposit: 100,
        p_apply_stage_length: 1200,
        p_commit_stage_length: 1200,
        p_reveal_stage_length: 1200,
        p_dispensation_percentage: 50,
        p_vote_quorum: 50,
    };

    let parameterizer_contract =
        Contract::deploy(web3.eth(), &compiler.load_abi("Parameterizer.abi"))
            .unwrap()
            .confirmations(0)
            .options(Options::with(|opt| opt.gas = Some(5_000_000.into())))
            .execute(
                parameterizer_bytecode,
                (
                    eip20_contract.address(),
                    plcr_contract.address(),
                    U256::from(config.min_deposit),
                    U256::from(config.p_min_deposit),
                    U256::from(config.apply_stage_length),
                    U256::from(config.p_apply_stage_length),
                    U256::from(config.commit_stage_length),
                    U256::from(config.p_commit_stage_length),
                    U256::from(config.reveal_stage_length),
                    U256::from(config.p_reveal_stage_length),
                    U256::from(config.dispensation_percentage),
                    U256::from(config.p_dispensation_percentage),
                    U256::from(config.vote_quorum),
                    U256::from(config.p_vote_quorum),
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
        .options(Options::with(|opt| opt.gas = Some(5_000_000.into())))
        .execute(
            registry_bytecode,
            (
                eip20_contract.address(),
                plcr_contract.address(),
                parameterizer_contract.address(),
            ),
            my_account,
        )
        .expect("Correct parameters to be passed into constructor");

    let registry_contract = pending.wait()
        .expect("Problem with registry deployment");

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


/// Makes application with the min deposit
pub fn add_listing<T>(web3: &web3::Web3<T>, info: &RegistryInfo<T>, name: &str)
    where T: Transport
{
    println!("Adding listing {}", name);

    // set up
    let accounts = web3.eth().accounts().wait().expect("Could not get accounts");
    // TODO: read from file
    let deposit = 10;
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

    let result = info.registry.query("isWhitelisted", (H256::from(res)), None, Options::default(), None);
    let added: bool = result.wait().expect("Problem checking whitelist");
    println!("isWhitelisted: {:?}", added);
}
