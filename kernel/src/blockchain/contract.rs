//! Smart Contract Execution Engine
//!
//! This module provides contract deployment, execution, ABI encoding/decoding,
//! and event logging functionality.

use crate::blockchain::{
    H256, Address, U256,
    evm::{EVM, ExecutionContext, ExecutionResult},
    error::ContractError,
};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::boxed::Box;

/// Contract ABI type
#[derive(Debug, Clone, PartialEq)]
pub enum ABIType {
    UInt,
    Int,
    Address,
    Bool,
    FixedBytes(usize),
    Bytes,
    String,
    Array(Box<ABIType>),
    FixedArray(Box<ABIType>, usize),
    Tuple(Vec<ABIType>),
}

/// Contract ABI function
#[derive(Debug, Clone)]
pub struct ABIFunction {
    pub name: String,
    pub inputs: Vec<(String, ABIType)>,
    pub outputs: Vec<(String, ABIType)>,
    pub is_payable: bool,
    pub is_view: bool,
    pub is_pure: bool,
}

/// Contract ABI event
#[derive(Debug, Clone)]
pub struct ABIEvent {
    pub name: String,
    pub inputs: Vec<(String, ABIType, bool)>, // (name, type, indexed)
    pub anonymous: bool,
}

/// Contract ABI definition
#[derive(Debug, Clone)]
pub struct ContractABI {
    pub functions: Vec<ABIFunction>,
    pub events: Vec<ABIEvent>,
    pub constructor: Option<ABIFunction>,
    pub fallback: bool,
    pub receive: bool,
}

impl ContractABI {
    /// Create new empty ABI
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            events: Vec::new(),
            constructor: None,
            fallback: false,
            receive: false,
        }
    }

    /// Find function by selector
    pub fn find_function(&self, selector: [u8; 4]) -> Option<&ABIFunction> {
        for func in &self.functions {
            let func_selector = Self::calculate_function_selector(func);
            if func_selector == selector {
                return Some(func);
            }
        }
        None
    }

    /// Calculate function selector (first 4 bytes of Keccak-256)
    pub fn calculate_function_selector(func: &ABIFunction) -> [u8; 4] {
        let signature = Self::function_signature(func);
        let hash = crate::blockchain::crypto::Keccak256::hash(signature.as_bytes());
        let mut selector = [0u8; 4];
        selector.copy_from_slice(&hash.0[..4]);
        selector
    }

    /// Generate function signature
    fn function_signature(func: &ABIFunction) -> String {
        let mut sig = func.name.clone();
        sig.push('(');

        for (i, (_, typ)) in func.inputs.iter().enumerate() {
            if i > 0 {
                sig.push(',');
            }
            sig.push_str(&Self::type_signature(typ));
        }

        sig.push(')');
        sig
    }

    /// Generate type signature
    fn type_signature(typ: &ABIType) -> String {
        match typ {
            ABIType::UInt => "uint256".to_string(),
            ABIType::Int => "int256".to_string(),
            ABIType::Address => "address".to_string(),
            ABIType::Bool => "bool".to_string(),
            ABIType::FixedBytes(size) => format!("bytes{}", size),
            ABIType::Bytes => "bytes".to_string(),
            ABIType::String => "string".to_string(),
            ABIType::Array(inner) => format!("{}[]", Self::type_signature(inner)),
            ABIType::FixedArray(inner, size) => format!("{}[{}]", Self::type_signature(inner), size),
            ABIType::Tuple(types) => {
                let mut sig = "(".to_string();
                for (i, t) in types.iter().enumerate() {
                    if i > 0 {
                        sig.push(',');
                    }
                    sig.push_str(&Self::type_signature(t));
                }
                sig.push(')');
                sig
            }
        }
    }
}

impl Default for ContractABI {
    fn default() -> Self {
        Self::new()
    }
}

/// Event log emitted by contract
#[derive(Debug, Clone, PartialEq)]
pub struct EventLog {
    /// Contract address
    pub address: Address,

    /// Topics (including event signature)
    pub topics: Vec<H256>,

    /// Event data
    pub data: Vec<u8>,

    /// Block number
    pub block_number: u64,

    /// Transaction hash
    pub transaction_hash: H256,

    /// Log index
    pub log_index: usize,
}

/// Transaction receipt
#[derive(Debug, Clone, PartialEq)]
pub struct TransactionReceipt {
    /// Whether transaction succeeded
    pub success: bool,

    /// Gas used
    pub gas_used: u64,

    /// Contract address (for creation)
    pub contract_address: Option<Address>,

    /// Logs
    pub logs: Vec<EventLog>,

    /// Cumulative gas used
    pub cumulative_gas_used: u64,

    /// State root
    pub state_root: H256,

    /// Status (1 = success, 0 = failure)
    pub status: u64,
}

/// Smart contract
#[derive(Debug, Clone)]
pub struct Contract {
    /// Contract address
    pub address: Address,

    /// Contract code
    pub code: Vec<u8>,

    /// Contract storage
    pub storage: BTreeMap<H256, H256>,

    /// Contract ABI
    pub abi: ContractABI,

    /// Contract balance
    pub balance: U256,

    /// Contract creator
    pub creator: Address,

    /// Nonce
    pub nonce: U256,
}

impl Contract {
    /// Create new contract
    pub fn new(
        address: Address,
        code: Vec<u8>,
        abi: ContractABI,
        creator: Address,
    ) -> Self {
        Self {
            address,
            code,
            storage: BTreeMap::new(),
            abi,
            balance: U256::ZERO,
            creator,
            nonce: U256::ZERO,
        }
    }

    /// Get storage slot
    pub fn get_storage(&self, key: H256) -> H256 {
        self.storage.get(&key).cloned().unwrap_or(H256::ZERO)
    }

    /// Set storage slot
    pub fn set_storage(&mut self, key: H256, value: H256) {
        self.storage.insert(key, value);
    }

    /// Get code hash
    pub fn code_hash(&self) -> H256 {
        crate::blockchain::crypto::Keccak256::hash(&self.code)
    }
}

/// Contract engine for managing contracts
#[derive(Debug)]
pub struct ContractEngine {
    /// Deployed contracts
    contracts: BTreeMap<Address, Contract>,

    /// Contract code database
    code_db: BTreeMap<H256, Vec<u8>>,

    /// Account balances
    balances: BTreeMap<Address, U256>,

    /// Account nonces
    nonces: BTreeMap<Address, U256>,

    /// Transaction count
    tx_count: u64,

    /// Block number
    block_number: u64,
}

impl ContractEngine {
    /// Create new contract engine
    pub fn new() -> Self {
        Self {
            contracts: BTreeMap::new(),
            code_db: BTreeMap::new(),
            balances: BTreeMap::new(),
            nonces: BTreeMap::new(),
            tx_count: 0,
            block_number: 0,
        }
    }

    /// Deploy new contract
    pub fn deploy(
        &mut self,
        creator: Address,
        bytecode: Vec<u8>,
        value: U256,
        gas_limit: u64,
    ) -> Result<(Address, TransactionReceipt), ContractError> {
        // Check creator has enough balance
        let creator_balance = self.balances.get(&creator).cloned().unwrap_or(U256::ZERO);
        if creator_balance < value {
            return Err(ContractError::InsufficientFunds);
        }

        // Calculate contract address (CREATE: address = keccak256(rlp(sender, nonce))[12:])
        let nonce = self.nonces.get(&creator).cloned().unwrap_or(U256::ZERO);
        let contract_address = Self::calculate_create_address(creator, nonce.clone());

        // Create execution context
        let mut ctx = ExecutionContext::new(
            creator,
            contract_address,
            value.clone(),
            Vec::new(),
            gas_limit,
        );
        ctx.code = bytecode.clone();

        // Execute constructor
        let mut evm = EVM::new(ctx);
        let result = evm.execute();

        // Update state
        let contract = Contract::new(
            contract_address,
            bytecode,
            ContractABI::new(),
            creator,
        );

        self.contracts.insert(contract_address, contract);
        self.balances.insert(contract_address, value.clone());

        // Update creator nonce and balance
        self.nonces.insert(creator, nonce.checked_add(&U256::ONE).unwrap_or(nonce));
        self.balances.insert(creator, creator_balance.checked_sub(&value).unwrap_or(creator_balance));

        let receipt = match result {
            ExecutionResult::Success { gas_used, .. } => TransactionReceipt {
                success: true,
                gas_used,
                contract_address: Some(contract_address),
                logs: Vec::new(),
                cumulative_gas_used: gas_used,
                state_root: H256::ZERO,
                status: 1,
            },
            ExecutionResult::Revert { gas_used, .. } => TransactionReceipt {
                success: false,
                gas_used,
                contract_address: None,
                logs: Vec::new(),
                cumulative_gas_used: gas_used,
                state_root: H256::ZERO,
                status: 0,
            },
            ExecutionResult::Error { gas_used, .. } => TransactionReceipt {
                success: false,
                gas_used,
                contract_address: None,
                logs: Vec::new(),
                cumulative_gas_used: gas_used,
                state_root: H256::ZERO,
                status: 0,
            },
        };

        Ok((contract_address, receipt))
    }

    /// Call contract function
    pub fn call(
        &mut self,
        caller: Address,
        contract_address: Address,
        input_data: Vec<u8>,
        value: U256,
        gas_limit: u64,
    ) -> Result<TransactionReceipt, ContractError> {
        // Check contract exists
        let contract = self.contracts.get(&contract_address)
            .ok_or(ContractError::ContractNotFound)?;

        // Check caller has enough balance
        let caller_balance = self.balances.get(&caller).cloned().unwrap_or(U256::ZERO);
        if caller_balance < value {
            return Err(ContractError::InsufficientFunds);
        }

        // Create execution context
        let mut ctx = ExecutionContext::new(
            caller,
            contract_address,
            value.clone(),
            input_data,
            gas_limit,
        );
        ctx.code = contract.code.clone();
        ctx.storage = contract.storage.clone();

        // Execute call
        let mut evm = EVM::new(ctx);
        let result = evm.execute();

        // Update contract storage
        if let Some(c) = self.contracts.get_mut(&contract_address) {
            c.storage = evm.storage().clone();
        }

        // Update balances
        if value > U256::ZERO {
            self.balances.insert(contract_address,
                self.balances.get(&contract_address).cloned().unwrap_or(U256::ZERO)
                    .checked_add(&value).unwrap_or(U256::ZERO)
            );
            self.balances.insert(caller,
                caller_balance.checked_sub(&value).unwrap_or(caller_balance)
            );
        }

        // Update caller nonce
        let nonce = self.nonces.get(&caller).cloned().unwrap_or(U256::ZERO);
        self.nonces.insert(caller, nonce.checked_add(&U256::ONE).unwrap_or(nonce));

        let receipt = match result {
            ExecutionResult::Success { gas_used, data: _ } => {
                // Parse logs from EVM
                let logs = Vec::new(); // Would extract from evm.logs()

                TransactionReceipt {
                    success: true,
                    gas_used,
                    contract_address: None,
                    logs,
                    cumulative_gas_used: gas_used,
                    state_root: H256::ZERO,
                    status: 1,
                }
            },
            ExecutionResult::Revert { gas_used, .. } => TransactionReceipt {
                success: false,
                gas_used,
                contract_address: None,
                logs: Vec::new(),
                cumulative_gas_used: gas_used,
                state_root: H256::ZERO,
                status: 0,
            },
            ExecutionResult::Error { gas_used, .. } => TransactionReceipt {
                success: false,
                gas_used,
                contract_address: None,
                logs: Vec::new(),
                cumulative_gas_used: gas_used,
                state_root: H256::ZERO,
                status: 0,
            },
        };

        Ok(receipt)
    }

    /// Get contract
    pub fn get_contract(&self, address: Address) -> Option<&Contract> {
        self.contracts.get(&address)
    }

    /// Get balance
    pub fn get_balance(&self, address: Address) -> U256 {
        self.balances.get(&address).cloned().unwrap_or(U256::ZERO)
    }

    /// Get nonce
    pub fn get_nonce(&self, address: Address) -> U256 {
        self.nonces.get(&address).cloned().unwrap_or(U256::ZERO)
    }

    /// Set balance (for testing/blockchain integration)
    pub fn set_balance(&mut self, address: Address, balance: U256) {
        self.balances.insert(address, balance);
    }

    /// Calculate CREATE address
    fn calculate_create_address(sender: Address, nonce: U256) -> Address {
        // Simplified - real implementation uses RLP encoding
        let mut data = [0u8; 52]; // sender (20) + nonce (variable, up to 32)
        data[..20].copy_from_slice(&sender.0);
        data[20..52].copy_from_slice(&nonce.to_bytes_be()[..32]);

        let hash = crate::blockchain::crypto::Keccak256::hash(&data);
        Address::from_h256(hash)
    }

    /// Calculate CREATE2 address
    pub fn calculate_create2_address(
        sender: Address,
        salt: H256,
        init_code_hash: H256,
    ) -> Address {
        let mut data = Vec::new();
        data.push(0xFF); // Prefix
        data.extend_from_slice(&sender.0);
        data.extend_from_slice(&salt.0);
        data.extend_from_slice(&init_code_hash.0);

        let hash = crate::blockchain::crypto::Keccak256::hash(&data);
        Address::from_h256(hash)
    }
}

impl Default for ContractEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// ABI Encoder
pub struct ABIEncoder;

impl ABIEncoder {
    /// Encode function call
    pub fn encode_function_call(func: &ABIFunction, args: &[ABIData]) -> Result<Vec<u8>, ContractError> {
        let selector = ContractABI::calculate_function_selector(func);
        let encoded_args = Self::encode_arguments(&func.inputs, args)?;

        let mut data = Vec::new();
        data.extend_from_slice(&selector);
        data.extend_from_slice(&encoded_args);

        Ok(data)
    }

    /// Encode arguments
    fn encode_arguments(inputs: &[(String, ABIType)], args: &[ABIData]) -> Result<Vec<u8>, ContractError> {
        if inputs.len() != args.len() {
            return Err(ContractError::ABIEncodeError);
        }

        let mut data = Vec::new();
        let mut head_size = 0;

        // Calculate head size
        for (_i, (_, typ)) in inputs.iter().enumerate() {
            if Self::is_dynamic_type(typ) {
                head_size += 32;
            } else {
                head_size += Self::type_size(typ);
            }
        }

        // Encode
        let mut offset = head_size;

        for (i, (_, typ)) in inputs.iter().enumerate() {
            if Self::is_dynamic_type(typ) {
                // Encode offset to tail
                data.extend_from_slice(&U256::from(offset as u64).to_bytes_be());

                // Encode data in tail section
                let encoded = Self::encode_data(typ, &args[i])?;
                data.extend_from_slice(&encoded);

                offset += encoded.len();
            } else {
                // Encode directly in head
                let encoded = Self::encode_data(typ, &args[i])?;
                data.extend_from_slice(&encoded);
            }
        }

        Ok(data)
    }

    /// Check if type is dynamic
    fn is_dynamic_type(typ: &ABIType) -> bool {
        matches!(typ, ABIType::Bytes | ABIType::String | ABIType::Array(_))
    }

    /// Get type size (static types only)
    fn type_size(typ: &ABIType) -> usize {
        match typ {
            ABIType::UInt | ABIType::Int | ABIType::Address | ABIType::Bool => 32,
            ABIType::FixedBytes(size) => *size,
            ABIType::FixedArray(inner, count) => Self::type_size(inner) * count,
            ABIType::Tuple(types) => types.iter().map(Self::type_size).sum(),
            _ => 32, // Dynamic types
        }
    }

    /// Encode single data item
    fn encode_data(typ: &ABIType, data: &ABIData) -> Result<Vec<u8>, ContractError> {
        match (typ, data) {
            (ABIType::UInt, ABIData::UInt(val)) => Ok(val.clone().to_bytes_be().to_vec()),
            (ABIType::Address, ABIData::Address(addr)) => {
                let mut bytes = [0u8; 32];
                bytes[12..].copy_from_slice(&addr.0);
                Ok(bytes.to_vec())
            }
            (ABIType::Bool, ABIData::Bool(b)) => {
                let val = if *b { U256::ONE } else { U256::ZERO };
                Ok(val.to_bytes_be().to_vec())
            }
            (ABIType::Bytes, ABIData::Bytes(data)) => {
                let mut encoded = Vec::new();
                encoded.extend_from_slice(&U256::from(data.len() as u64).to_bytes_be());

                // Pad to 32-byte boundary
                let padded_len = (data.len() + 31) / 32 * 32;
                encoded.extend_from_slice(data);
                encoded.resize(padded_len, 0);

                Ok(encoded)
            }
            _ => Err(ContractError::ABIEncodeError),
        }
    }
}

/// ABI Decoder
pub struct ABIDecoder;

impl ABIDecoder {
    /// Decode function return value
    pub fn decode_function_output(func: &ABIFunction, data: &[u8]) -> Result<Vec<ABIData>, ContractError> {
        if data.len() < 4 {
            return Err(ContractError::ABIDecodeError);
        }

        let output_data = &data[4..]; // Skip selector
        Self::decode_arguments(&func.outputs, output_data)
    }

    /// Decode arguments
    fn decode_arguments(outputs: &[(String, ABIType)], data: &[u8]) -> Result<Vec<ABIData>, ContractError> {
        let mut results = Vec::new();
        let mut offset = 0;

        for (_, typ) in outputs {
            let decoded = Self::decode_data(typ, data, &mut offset)?;
            results.push(decoded);
        }

        Ok(results)
    }

    /// Decode single data item
    fn decode_data(typ: &ABIType, data: &[u8], offset: &mut usize) -> Result<ABIData, ContractError> {
        match typ {
            ABIType::UInt => {
                let val = U256::from_bytes_be(
                    data[*offset..*offset + 32].try_into().map_err(|_| ContractError::ABIDecodeError)?
                );
                *offset += 32;
                Ok(ABIData::UInt(val))
            }
            ABIType::Address => {
                let mut addr_bytes = [0u8; 20];
                addr_bytes.copy_from_slice(&data[*offset + 12..*offset + 32]);
                *offset += 32;
                Ok(ABIData::Address(Address::new(addr_bytes)))
            }
            ABIType::Bool => {
                let val = U256::from_bytes_be(
                    data[*offset..*offset + 32].try_into().map_err(|_| ContractError::ABIDecodeError)?
                );
                *offset += 32;
                Ok(ABIData::Bool(!val.is_zero()))
            }
            _ => Err(ContractError::ABIDecodeError),
        }
    }
}

/// ABI data value
#[derive(Debug, Clone, PartialEq)]
pub enum ABIData {
    UInt(U256),
    Int(i256),
    Address(Address),
    Bool(bool),
    FixedBytes(Vec<u8>),
    Bytes(Vec<u8>),
    String(String),
    Array(Vec<ABIData>),
}

// Simplified i256 type
#[derive(Debug, Clone, PartialEq)]
pub struct i256(pub U256);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deploy_contract() {
        let mut engine = ContractEngine::new();
        let creator = Address::new([1u8; 20]);
        let bytecode = vec![0x60, 0x01, 0x60, 0x01, 0x01]; // ADD operation

        engine.set_balance(creator, U256::from(1000));

        let (addr, receipt) = engine.deploy(creator, bytecode, U256::ZERO, 100000).unwrap();

        assert!(receipt.success);
        assert_eq!(engine.get_contract(addr).is_some(), true);
    }

    #[test]
    fn test_call_contract() {
        let mut engine = ContractEngine::new();
        let caller = Address::new([1u8; 20]);
        let contract_addr = Address::new([2u8; 20]);

        // Deploy contract
        let bytecode = vec![Opcode::STOP as u8];
        engine.deploy(caller, bytecode, U256::ZERO, 100000).unwrap();

        // Call contract
        engine.set_balance(caller, U256::from(1000));
        let receipt = engine.call(caller, contract_addr, vec![], U256::ZERO, 100000).unwrap();

        assert!(receipt.success);
    }

    #[test]
    fn test_abi_encoding() {
        let func = ABIFunction {
            name: "transfer".to_string(),
            inputs: vec![
                ("recipient".to_string(), ABIType::Address),
                ("amount".to_string(), ABIType::UInt),
            ],
            outputs: vec![("success".to_string(), ABIType::Bool)],
            is_payable: false,
            is_view: false,
            is_pure: false,
        };

        let args = vec![
            ABIData::Address(Address::new([1u8; 20])),
            ABIData::UInt(U256::from(100)),
        ];

        let encoded = ABIEncoder::encode_function_call(&func, &args).unwrap();
        assert_eq!(encoded.len(), 4 + 32 + 32); // selector + 2 args
    }

    #[test]
    fn test_balance_and_nonce() {
        let mut engine = ContractEngine::new();
        let addr = Address::new([1u8; 20]);

        assert_eq!(engine.get_balance(addr), U256::ZERO);
        assert_eq!(engine.get_nonce(addr), U256::ZERO);

        engine.set_balance(addr, U256::from(1000));

        assert_eq!(engine.get_balance(addr), U256::from(1000));
    }
}
