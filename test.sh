#!/bin/bash

# Example query that fetches some fee history data from the Ethereum network.

payload='{"jsonrpc":"2.0","method":"eth_feeHistory","id":97,"params":["0x4","latest",[]]}'

curl -i -X POST "http://localhost:8787/sepolia" \
     -H "Content-Type: application/json" \
     -d "$payload"
