# To test in developtment
## Set up soroban identities (Only once)
```
soroban config identity add alice
soroban config identity add bob
```

# Build the contract
```
soroban contract build
```
or
```
cargo build --target wasm32-unknown-unknown --release
```

# To Deploy

```
soroban contract deploy --wasm target/wasm32-unknown-unknown/release/swap_contract.wasm --network testnet --source alice
```
Returns the Contract ID ex: `CASIPFCJIRH5BLAJKLY6KNYXGP6JOI4DGMTBBX7D7OHO32PGPLPEYFNG`


# Initialize
```
contract_id=CA7M3K4Q2GDQML6354N4ZSJW42R2G33IIV3MTX67UKXHWGVWBWEMZTHR \
token_a=CAXU27NOCRBFTNUPR7ROLD4CAAHBQ55Z6T7GCREPRFDR5ED2PCVR5LFQ \
token_b=CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC \

soroban contract invoke --id $contract_id --network testnet --source alice -- initialize --admin alice --token_a $token_a --token_b $token_b --name_token_a USDC --name_token_b EURC --forward_rate 100000000000000 --duration 604800


soroban contract invoke --id $contract_id --network testnet -- initialize --admin  --token_a CAWH4XMRQL7AJZCXEJVRHHMT6Y7ZPFCQCSKLIFJL3AVIQNC5TSVWKQOR --token_b CCBINL4TCQVEQN2Q2GO66RS4CWUARIECZEJA7JVYQO3GVF4LG6HJN236 $token_b --name_token_a USDC --name_token_b EURC --forward_rate 100000000000000 --duration 604800
```

## Common tokens for testnet
```
XLM  = CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC
USDC = CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA
EURC = CCUUDM434BMZMYWYDITHFXHDMIVTGGD6T2I5UKNX5BSLXLW7HVR4MCGZ
```
## Wrap Stellar Asset into Soroban
USDC token is issued by `GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5`, to use it within a soroban contract we must wrap it:
```
soroban lab token wrap --network standalone --source token-admin --asset "ASSET_CODE:ISSUER_ADDRESS"
```
For example with USDC will be:
```
soroban lab token wrap --network testnet --source alice --asset "USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5"
```

# Set up Positions
```
soroban contract invoke --id $contract_id --network testnet --source alice -- init_pos --from alice --positions_token_a 2 --positions_token_b 2 --amount_deposit_token_a 1000000
```


# Deposit
```
soroban contract invoke --id $contract_id --network testnet --source alice -- deposit --from alice --token $token_a --amount 1000000 --collateral 200000

soroban contract invoke --id $contract_id --network testnet --source bob -- deposit --from bob --token $token_b --amount 1000000 --collateral 200000
```

# Execute near leg (Optional, only if there was an error during initialization)
```
soroban contract invoke --id $contract_id --network testnet -- near_leg
```

# Get Spot Rate
```
soroban contract invoke --id $contract_id --source alice --network testnet -- spot_rate
```

# Swap Assets
```
soroban contract invoke --id $contract_id --network testnet --source alice -- swap --from alice

soroban contract invoke --id $contract_id --network testnet --source bob -- swap --from bob
```

# Repay Asset
```
soroban contract invoke --id $contract_id --network testnet --source alice -- repay --from alice --token $token_b --amount 10000000

soroban contract invoke --id $contract_id --network testnet --source bob -- repay --from bob --token $token_a --amount 10000000
```

# Withdraw Original Asset
```
soroban contract invoke --id $contract_id --network testnet --source alice -- withdraw --from alice

soroban contract invoke --id $contract_id --network testnet --source bob -- withdraw --from bob
```

# Reclaim unused deposit
```
soroban contract invoke --id $contract_id --network testnet --source alice -- reclaim --from alice

soroban contract invoke --id $contract_id --network testnet --source bob -- reclaim --from bob
```

# Reclaim Collateral
```
soroban contract invoke --id $contract_id --network testnet --source alice -- reclaim_col --from alice

soroban contract invoke --id $contract_id --network testnet --source bob -- reclaim_col --from bob
```

# Liquidate User
```
soroban contract invoke --id $contract_id --network testnet --source alice -- liquidate --from alice --to bob
```

-----------------------
# Install WASM to use in the deployer
```
soroban contract install --wasm ./target/wasm32-unknown-unknown/release/swap_contract.wasm --network testnet --source alice
```
Returns contract wasm ex: `d6000267f42d63bb6c845cc62bd616d11d446bc97b2b7ec25a2c43e98d4307f0`

# Deploy using the deployer
```
soroban contract invoke \
    --id CAMWD3ZGESCWMN2SGC2UC4JWI3BBURVWUF46JIHJSAZNRTX32X4NFRRQ \
    --source alice \
    --network testnet \
    -- deploy \
    --salt 0000000000000000000000000000000000000000000000000000000000000000 \    --deployer CAMWD3ZGESCWMN2SGC2UC4JWI3BBURVWUF46JIHJSAZNRTX32X4NFRRQ \
    --wasm_hash ed24fd3c946e6f9830e240f28333533d6527c0f4e404665b67b39e75a8b4a51a \
    --init_fn initialize
    --init_args '[{"address":"GD324GL3IVXNY4GR4JOWINAD3RHYNVYN4LT4HH4QC7CTHN7ZJBHU4AEX"},{"address":"CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA"},{"address":"CCUUDM434BMZMYWYDITHFXHDMIVTGGD6T2I5UKNX5BSLXLW7HVR4MCGZ"},
    {"symbol":"USDC"},
    {"symbol":"EURC"},
    {"i128":[1000000,0]},
    {"u64":3600}
    ]'
```
------------------------
# Using custom tokens
------------------------
# Deploy token
```
soroban contract deploy --wasm target/wasm32-unknown-unknown/release/soroban_token_contract.wasm --source alice --network testnet
```

# Initialize token
```
soroban contract invoke --id CBNPDIIALURDPGRYFYSNDF4P2X2FANRESNP7OUT67VZEQZ3A4I26RB7I --network testnet -- initialize --admin alice --decimal 8 --name koken --symbol kok
```

# Mint token
```
soroban contract invoke --id CBNPDIIALURDPGRYFYSNDF4P2X2FANRESNP7OUT67VZEQZ3A4I26RB7I --network testnet -- mint --to alice --amount 100000000000000000000000000
```

# View user balance
```
soroban contract invoke --id CBNPDIIALURDPGRYFYSNDF4P2X2FANRESNP7OUT67VZEQZ3A4I26RB7I --network testnet -- balance --id alice
```

---------
# Costs of transactions
The more information store in the instance storage, the more expensive will be each transaction, using persistent data will also make the transaction expensier, but only in those functions that require access persistent data.


---------
# Storage Types

## Ledgers
A ledger represents the state of the Stellar network at a point in time.
Data is stored on the ledger as ledger entries.

## TTL
All contract data has a Time To Live (TTL), measured in ledgers, that must be periodically extended. If an entry's TTL is not periodically extended, the entry will eventually become "archived".
The default lifetime of a new persistent entry is just 86400 ledgers (~5 days) and just 16 ledgers for a temp entry (~1.5m)
100 ledgers are about 500 seconds.

## Instance Data

All instance storage is kept in a single contract instance called LedgerEntry, with a 64KB size. Anything stored in instance storage has an archival TTL that is tied to the contract instance itself. Therefore, if a contract is live and available, the instance storage is guaranteed to be so as well.

## Persistent Data
the instance storage works the same as persistent storage, except its own TTL is tied to that of the contract instance.

Persistent data types are not retrieved every time the contract is called. Hence, the cost of reading the contract does not increase for unrelated functions.

While having unlimited amount of storage, using the Persistent Data Type to store an array is still limited to 64KB of information.

## Array of Deposits

Using an array to store the deposits, given the limit of 64KB, and considering each position is 48 bytes, we can store a maximum amount of 1365 positions.
Though, we need one array for each currency, implying that each amount will be at maximum 682 positions. 

## Extend a deployed contract instance's TTL
### From the CLI
```
soroban contract extend \
    --source alice \
    --network testnet \
    --id $contract_id \
    --ledgers-to-extend 535679 \
    --durability persistent
``` 
This example uses 535,679 ledgers as the new archival TTL. This is the maximum allowable value for this argument on the CLI. This corresponds to roughly 30 days (averaging 5 second ledger close times).

With this we can extend any `env.storage().instance()` entries in the contract

### From the JS SDK [see](https://developers.stellar.org/docs/fundamentals-and-concepts/list-of-operations#extend-footprint-ttl)
can be restored after archival using the RestoreFootprintOp operation.

