import web3
from eth_account import Account
import json
from web3 import Web3
from logger import logger

class ContractHandler:
    def __init__(self, private_key: str, contract_address: str, contract_abi_path: str, rpc_url: str, chain_id: str):
        self.node = Web3(Web3.HTTPProvider(rpc_url))
        self.private_key = private_key
        self.account = Account.from_key(private_key)
        with open(contract_abi_path, "r") as f:
            contract_abis = json.load(f)
            contract_abi = contract_abis["TwitterAgent"]
        self.contract_details = self.node.eth.contract(address=contract_address, abi=contract_abi)

    def emit_event(self, twitter_comment_id: str):
        nonce = self.node.eth.get_transaction_count(self.account.address)
        transaction = self.contract_details.functions.twitterEmmiter(twitter_comment_id).build_transaction({
            'from': self.account.address,
            'gas': 500000,
            'nonce': nonce
        })
        signed_txn = self.node.eth.account.sign_transaction(transaction, self.private_key)
        tx_hash = self.node.eth.send_raw_transaction(signed_txn.rawTransaction)
        return tx_hash
    
    # Utility function to read events from the contract
    def read_event(self):
        event_signature = self.node.keccak(text="TwitterEmitter(string)").hex()
        logs = self.node.eth.get_logs({
            "fromBlock": "earliest",
            "toBlock": "latest",
            "address": self.contract_details.address,
            "topic": event_signature
        })
        
        for log in logs:
            twitter_comment_id = self.node.to_text(log["data"]).strip('\x00').strip().lstrip('\x00').strip()
            logger.info("Event emitted: Twitter Comment ID: %s", twitter_comment_id)
