# launch bitcoind and ord
ord --bitcoin-rpc-username foo --bitcoin-rpc-password bar env ord_server_4

ord --datadir ord_server_4 wallet receive
# bcrt1pcj3t3dzey9u3vhsvjnwv6nlge0f2kh0wk07jjx4vm9kr9ng4xdtq3wkkam
bitcoin-cli -datadir=ord_server_4 generatetoaddress 101 bcrt1pcj3t3dzey9u3vhsvjnwv6nlge0f2kh0wk07jjx4vm9kr9ng4xdtq3wkkam
ord --datadir ord_server_4 wallet inscribe --fee-rate 1 --file ord_server_4/batch.yaml
bitcoin-cli -datadir=ord_server_4 generatetoaddress 100 bcrt1pcj3t3dzey9u3vhsvjnwv6nlge0f2kh0wk07jjx4vm9kr9ng4xdtq3wkkam

ord --datadir ord_server_4 wallet cardinals


ord --datadir ord_server_4 wallet send --fee-rate 1 bcrt1ph597m7u6qc5rfzanl9fsvxfvayyedwtuh34gmvadqnd0gm4vg48qaf907s 10:FOOFOO

ord --datadir ord_server_4 wallet batch --fee-rate 1 --batch "ord_server_4/batch2.yaml"


# bitcoin-cli -datadir=ord_server_4 getblock "741b7ffaade15a0c6972d7b3e14e339fafc1d745733031c34e449742aafb2222" 2

curl --user foo --data-binary '{"jsonrpc": "1.0", "id": "curltest", "method": "getblock", "params": ["741b7ffaade15a0c6972d7b3e14e339fafc1d745733031c34e449742aafb2222"]}' -H 'content-type: text/plain;' http://127.0.0.1:9001/
bitcoin-cli -datadir=ord_server_4 generatetoaddress 20 bcrt1pcj3t3dzey9u3vhsvjnwv6nlge0f2kh0wk07jjx4vm9kr9ng4xdtq3wkkam

ord --datadir ord_server_4 wallet batch --fee-rate 1 --batch ord_server_4/batch2.yaml