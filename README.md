# Utu Backend (Runes Bridge)

This repository hosts the backend service for the Utu Runes Bridge, a centralized component that facilitates bridging Runes from Bitcoin to Starknet. The service manages deposit address generation and provides claiming signatures. While this service manages the bridge operations, it does not have custody of the deposited bitcoins, which are controlled by a federation.

## Key Components

### 1. Bitcoin Deposit Address Generation
- Generates deterministic Bitcoin deposit addresses linked to Starknet addresses
- Uses [utu_bridge_deposit_address](https://github.com/lfglabs-dev/utu_bridge_deposit_address) for address generation
- Stores address mappings in MongoDB

### 2. Deposit Monitoring
- Monitors Bitcoin deposits using Hiro's Runes API
- Tracks deposit status (pending, confirmed, claimed)
- Associates deposits with corresponding Starknet addresses

### 3. Claim Signature Generation
- Validates deposit claims
- Generates signatures required for minting debt recognition tokens on Starknet
- Does not control deposited bitcoins (federation-controlled)

## API Endpoints

### Address Generation
```
GET /get_bitcoin_deposit_addr/{starknet_address}
```
Generates a Bitcoin deposit address for a given Starknet address (and stores the mapping in db if not done already).

Response:
```json
{
    "bitcoin_address": "bc1p...",
    "starknet_address": "0x..."
}
```

### Deposit Queries
```
GET /get_deposits/bitcoin?bitcoin_address=xxx
GET /get_deposits/starknet?starknet_address=xxx
```
Query deposits and their status for either a Bitcoin or Starknet address.

Response:
```json
{
    "status": "success",
    "data": {
        "pending": [
            {
                "txid": "...",
                "block_height": 123456,
                "block_hash": "...",
                "timestamp": 1234567890,
                "runes": [
                    {
                        "id": "...",
                        "amount": "...",
                        "claimed": false
                    }
                ]
            }
        ],
        "confirmed": [
            {
                "txid": "...",
                "block_height": 123456,
                "block_hash": "...",
                "timestamp": 1234567890,
                "runes": [
                    {
                        "id": "...",
                        "amount": "...",
                        "claimed": false
                    }
                ]
            }
        ],
        "claimed": [
            {
                "txid": "...",
                "block_height": 123456,
                "block_hash": "...",
                "timestamp": 1234567890,
                "runes": [
                    {
                        "id": "...",
                        "amount": "...",
                        "claimed": true
                    }
                ]
            }
        ]
    }
}
```

The deposits are grouped by their status: `pending`, `confirmed`, or `claimed`. Each status contains an array of deposit activities with their associated runes information.

### Claim Data
```
GET /claim_deposit_data/{txid}
```
Returns the required data to forge a claim transaction for a specific deposit.

Response:
```json
{
    "rune_id": "...",
    "amount": "...",
    "proof": {
        // Proof structure as required by the contract
    }
}
```

## Running the Application

1. Obtain the `.env.keys` file or update the `.env` file with necessary configurations.
2. Run the following command to start the REST server:
   ```bash
   dotenvx run -- cargo run
   ```
   Or for production:
   ```bash
   dotenvx run -f .env.production -- cargo run
   ```

## Database Collections

### deposit_addresses
```json
{
    "starknet_address": "0x...",
    "bitcoin_deposit_address": "bc1p...",
    "created_at": "timestamp"
}
```

### supported_runes
```json
{
    "id": "rune_id",
    "name": "Rune Name",
    "spaced_name": "Rune Name",
    "number": 123,
    "divisibility": 8,
    "symbol": "RUNE",
    "turbo": false,
    "mint_terms": {},
    "supply": {},
    "location": {}
}
```

## Dependencies
- MongoDB for address mapping storage
- [Hiro Runes API](https://docs.hiro.so/bitcoin/runes/api/activities/) for deposit monitoring
- [utu_bridge_deposit_address](https://github.com/lfglabs-dev/utu_bridge_deposit_address) for address generation

## Security Considerations
This service is a centralized component that must be operated by a trusted party. While it manages bridge operations, the actual bitcoins remain under federation control through a separate multisig setup.
