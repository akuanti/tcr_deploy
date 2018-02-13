extern crate rustc_hex;
extern crate serde_json;
extern crate tcr_deploy;
extern crate web3;

use std::env;
use std::fmt::Debug;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use web3::futures::Future;
use web3::futures::future::ok;
use web3::contract::{Contract, Options};
use web3::types::{Address, U256};
use web3::Transport;

use rustc_hex::FromHex;

use tcr_deploy::{compile, Library};

//deploy token
//deploy dll
//deploy atrstore
//link/deploy PLCRVoting
//link/deploy Parameterizer
//link/deploy Registry

// TODO: abstract Command as part of Solc

// TODO: return Result
fn compile_libraries(compiler: &compile::Solc) {
    println!("compiling in {}", compiler.root());
    println!("output dir: {}", compiler.output_dir());
    // should output to tcr/<output>
    // const TCR_PATH: &str = "tcr";
    let output_dir = format!("{}/{}", compiler.root(), compiler.output_dir());

    let mut abs_path: PathBuf = env::current_dir().unwrap();
    abs_path.extend(&["../tcr", "installed_contracts"]);
    let abs_path: PathBuf = abs_path.canonicalize().unwrap();
    let allow_paths = vec!["--allow-paths", abs_path.as_os_str().to_str().unwrap()];

    // binaries only for libraries
    let output = Command::new("solc")
        .current_dir(compiler.root())
        .arg("--bin")
        // TODO: add this to compiler
        .args(&allow_paths)
        .arg("-o")
        .arg(output_dir)
        // always overwrite initial library bytecode
        .arg("--overwrite")
        // sources
        .arg("installed_contracts/dll/contracts/DLL.sol")
        .arg("installed_contracts/attrstore/contracts/AttributeStore.sol")
        .output()
        .expect("failed to execute process");

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

// TODO: return Result
// fn compile_contracts(contract_root: &str, lib_file: &Path, output_dir: &str) {
fn compile_contracts<P>(compiler: &compile::Solc, lib_file_path: P)
where
    P: AsRef<Path>,
{
    println!("compiling contracts");
    // compile and link dependent contracts
    // PLCRVoting -> EIP20, [DLL], [AttributeStore]
    // Parameterizer -> PLCRVoting, [EIP20]
    // Registry -> EIP20, Parameterizer, PLCRVoting

    let tcr_path = compiler.root();

    let installed_path = "installed_contracts";
    // println!("CONTRACT INSTALL PATH: {:?}", installed_path);

    let mut abs_path: PathBuf = env::current_dir().unwrap();
    abs_path.extend(&[tcr_path, installed_path]);
    let abs_path: PathBuf = abs_path.canonicalize().unwrap();
    let allow_paths = vec!["--allow-paths", abs_path.as_os_str().to_str().unwrap()];
    // println!("ALLOW {:?}", allow_paths);

    // paths to installed libraries
    // <tcr_path>/installed_contracts/<package>/contracts
    let lib_paths: Vec<String> = ["dll", "attrstore", "tokens"]
        .iter()
        .map(|package| format!("{}={}/{}/contracts", package, installed_path, package))
        .collect();
    // println!("LIBS {:?}", lib_paths);

    // input Solidity files
    let sources: Vec<String> = ["PLCRVoting.sol", "Parameterizer.sol", "Registry.sol"]
        .iter()
        .map(|src| format!("contracts/{}", src))
        .collect();
    println!("SOURCES {:?}", &sources[..]);

    // output dir: <tcr>/<output\>
    let output_dir: PathBuf = [compiler.root(), compiler.output_dir.unwrap()]
        .iter()
        .collect();
    println!("OUTPUT: {:?}", output_dir);

    let mut cmd = Command::new("solc");
    cmd.current_dir(tcr_path)
        .arg("--bin")
        .arg("--abi")
        .args(&allow_paths)
        .args(&lib_paths)
        .arg("--libraries")
        .arg(lib_file_path.as_ref())
        .arg("-o")
        .arg(output_dir)
        .args(&sources);

    println!("{:?}", cmd);

    let output = cmd.output().expect("failed to execute process");

    println!("status: {}", output.status);
    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // TODO: ignore "refusing to overwrite"
    // if "Refusing to overwrite" in stderr, count as ok
    // if !output.status.success() {
    //     panic!("Compliation failed");
    // }
}

#[derive(Debug)]
struct Parameters {
    min_deposit: U256,
    apply_stage_length: U256,
    commit_stage_length: U256,
    reveal_stage_length: U256,
    dispensation_percentage: U256,
    vote_quorum: U256,
    // parameterizer
    p_min_deposit: U256,
    p_apply_stage_length: U256,
    p_commit_stage_length: U256,
    p_reveal_stage_length: U256,
    p_dispensation_percentage: U256,
    p_vote_quorum: U256,
}

fn main() {
    let (_eloop, http) = web3::transports::Http::new("http://localhost:8545").unwrap();
    let web3 = web3::Web3::new(http);

    let my_account: Address = web3.eth().accounts().wait().unwrap()[0];

    // optional, compile everything
    // contract root = HERE
    let contract_root = env::current_dir().unwrap();
    let contract_root = contract_root.to_str().unwrap();
    println!("ROOT: {:?}", contract_root);

    // relative to tcr root
    const BUILD_DIR: &str = "some_place";

    let mut compiler = compile::Solc::new("../tcr");
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
    // TODO: pass in compiler
    compile_contracts(&compiler, &lib_file);

    // deploy contracts (in order)
    // EIP20
    // PLCRVoting
    // Parameterizer
    // Registry
    println!("deploying contracts");

    // Token
    let eip20_bytecode: Vec<u8> = compiler.load_bytecode("EIP20.bin"); //include_str!("../tcr/some_place/EIP20.bin").from_hex().unwrap();

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
        )
        .expect("Correct parameters are passed to the constructor.")
        .wait()
        .unwrap();

    println!("EIP20:{:?}", eip20_contract.address());

    // PLCR Voting
    let plcr_bytecode: Vec<u8> = compiler.load_bytecode("PLCRVoting.bin");
    let plcr_contract = Contract::deploy(web3.eth(), &compiler.load_abi("PLCRVoting.abi"))
        .unwrap()
        .confirmations(0)
        .options(Options::with(|opt| opt.gas = Some(1_000_000.into())))
        .execute(plcr_bytecode, (eip20_contract.address(),), my_account)
        .expect("Correct parameters are passed to the constructor.")
        .wait()
        .unwrap();

    println!("PLCRVoting:{:?}", plcr_contract.address());

    // Paraeterizer
    let parameterizer_bytecode: Vec<u8> = compiler.load_bytecode("Parameterizer.bin");
    // TODO: read from file
    let config = Parameters {
        min_deposit: 10.into(),
        apply_stage_length: 600.into(),
        commit_stage_length: 600.into(),
        reveal_stage_length: 600.into(),
        dispensation_percentage: 50.into(),
        vote_quorum: 50.into(),
        p_min_deposit: 100.into(),
        p_apply_stage_length: 1200.into(),
        p_commit_stage_length: 1200.into(),
        p_reveal_stage_length: 1200.into(),
        p_dispensation_percentage: 50.into(),
        p_vote_quorum: 50.into(),
    };

    let parameterizer_contract =
        Contract::deploy(web3.eth(), &compiler.load_abi("Parameterizer.abi"))
            .unwrap()
            .confirmations(0)
            .options(Options::with(|opt| opt.gas = Some(1_000_000.into())))
            .execute(
                parameterizer_bytecode,
                (
                    eip20_contract.address(),
                    plcr_contract.address(),
                    config.min_deposit,
                    config.p_min_deposit,
                    config.apply_stage_length,
                    config.p_apply_stage_length,
                    config.commit_stage_length,
                    config.p_commit_stage_length,
                    config.reveal_stage_length,
                    config.p_reveal_stage_length,
                    config.dispensation_percentage,
                    config.p_dispensation_percentage,
                    config.vote_quorum,
                    config.p_vote_quorum,
                ),
                my_account,
            )
            .expect("Correct parameters to be passed into constructor")
            .wait()
            .unwrap();

    println!("Parameterizer:{:?}", parameterizer_contract.address());

    // Registry
    let registry_bytecode: Vec<u8> = compiler.load_bytecode("Registry.bin");
    let registry_contract = Contract::deploy(web3.eth(), &compiler.load_abi("Registry.abi"))
        .unwrap()
        .confirmations(0)
        .options(Options::with(|opt| opt.gas = Some(1_000_000.into())))
        .execute(
            registry_bytecode,
            (
                eip20_contract.address(),
                plcr_contract.address(),
                parameterizer_contract.address(),
            ),
            my_account,
        )
        .expect("Correct parameters to be passed into constructor")
        .wait()
        .unwrap();

    println!("Registry:{:?}", registry_contract.address());
}
