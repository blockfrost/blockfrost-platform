# TX evaluator

This package contains native and external tx evaluator.

- Native is based on `pallas-validate`.
- External is using `ledger` codebase via `testgen` Haskell binary.

`native` produces different evaluation results when compared to `external` at the moment. Knowing that `external` depends on the ledger codebase, we can consider `external` results accurate.

## Versions

Both `native` and `external` evaluator produces version 5 & 6 results. These versions are there for compatibility with [Blockfrost](https://docs.blockfrost.io/#tag/cardano--utilities/POST/utils/txs/evaluate). Blockfrost implementation is a proxy to Ogmios, so version 5 & 6 is Ogmios versions.

### Version 5 Data Structure

#### V5 Full Input for API

```javascript
{
  "type": "jsonwsp/request",
  "version": "1.0",
  "servicename": "ogmios",
  "methodname": "EvaluateTx",
  "args": {
    "evaluate": "string",
    "additionalUtxoSet": [
      [
        {
          "txId": "stringstringstringstringstringstringstringstringstringstringstri",
          "index": 4294967295
        },
        {
          "address": "addr_test1qz66ue36465w2qq40005h2hadad6pnjht8mu6sgplsfj74qdjnshguewlx4ww0eet26y2pal4xpav5prcydf28cvxtjqx46x7f",
          "value": {
            "coins": 2,
            "assets": {
              "3542acb3a64d80c29302260d62c3b87a742ad14abf855ebc6733081e": 42,
              "b5ae663aaea8e500157bdf4baafd6f5ba0ce5759f7cd4101fc132f54.706174617465": 1337
            }
          },
          "datumHash": null,
          "datum": null,
          "script": null
        }
      ]
    ]
  },
  "mirror": null
}
```

#### V5 Full Output for API

```javascript
{
  "type": "jsonwsp/response",
  "version": "1.0",
  "servicename": "ogmios",
  "methodname": "EvaluateTx",
  "result": {
    "spend:1": {
      "memory": 5236222,
      "cpu": 1212353
    },
    "mint:0": {
      "memory": 5000,
      "cpu": 42
    }
  },
  "reflection": null
}
```

### Version 6 Data Structure

#### V6 Full Input for API

```javascript
{
  "jsonrpc": "2.0",
  "method": "evaluateTransaction",
  "params": {
    "transaction": {
      "cbor": "string"
    },
    "additionalUtxo": [
      {
        "transaction": {
          "id": "stringstringstringstringstringstringstringstringstringstringstri"
        },
        "index": 4294967295,
        "address": "addr1q9d34spgg2kdy47n82e7x9pdd6vql6d2engxmpj20jmhuc2047yqd4xnh7u6u5jp4t0q3fkxzckph4tgnzvamlu7k5psuahzcp",
        "value": {
          "ada": {
            "lovelace": 0
          },
          "property1": {
            "property1": 0,
            "property2": 0
          },
          "property2": {
            "property1": 0,
            "property2": 0
          }
        },
        "datumHash": "c248757d390181c517a5beadc9c3fe64bf821d3e889a963fc717003ec248757d",
        "datum": "string",
        "script": {
          "language": "native",
          "json": {
            "clause": "signature",
            "from": "3c07030e36bfff7cd2f004356ef320f3fe3c07030e7cd2f004356437"
          },
          "cbor": "string"
        }
      }
    ]
  },
  "id": null
}
```

#### V6 Full Output for API

```javascript
{
  "jsonrpc": "2.0",
  "result": [
    {
      "validator": "spend:1",
      "budget": {
        "memory": 5236222,
        "cpu": 1212353
      }
    },
    {
      "validator": "mint:0",
      "budget": {
        "memory": 5000,
        "cpu": 42
      }
    }
  ]
}

```
