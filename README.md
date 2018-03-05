# TCR Deployer

This is a script for compiling and deploying a [token-curated registry](https://github.com/skmgoldin/tcr). It is intended for **testing purposes only!**

To just deploy a TCR, adjust the params in `conf/config.json`, then run the main script with cargo:
```
cargo run
```

You can also import it as a library and pass in the lpath to your configuration file to `deploy()`.

## Requirements
You must have a Solidity compiler installed and on your `PATH`. You must also have an Ethereum node to connect to.

## Configuration
Specify the parameters for the TCR's parameterizer, and compile/deployment options.

the directory with the TCR code, and the directory to store the compiled files.

All fields are required except `gasLimit` and `listingDeposit`.

```json
{
  // parameterizer
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
  // the directory with the TCR code
  "tcrDirectory": "../tcr",
  // the directory to store the compiled files
  "compilerBuildDirectory": "test_build",
  // OPTIONAL: gas limit to use for deployments
  "gasLimit": 3000000,
  // OPTIONAL: deposit to use for adding listings
  "listingDeposit": 20,
  // listings to add
  "listings": [
      "abc.com",
      "def.com"
  ]
}
```

