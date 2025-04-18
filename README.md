# Arch Network Smart Contract System

## Overview

The Arch Network Smart Contract System is a Rust-based implementation designed to facilitate Bitcoin pooling, proposal voting, and fund distribution. This system enables users to create pools, contribute Bitcoin, submit proposals, vote on proposals, and execute transfers to winning proposals.

## Table of Contents

- [Features](#features)
- [Architecture](#architecture)
- [Import Structure](#import-structure)
- [Installation](#installation)
- [Usage](#usage)
- [Contract Lifecycle](#contract-lifecycle)
- [API Reference](#api-reference)
- [Testing](#testing)
- [Security Considerations](#security-considerations)
- [Contributing](#contributing)
- [License](#license)

## Features

- **Pool Management**: Create and manage Bitcoin contribution pools with configurable parameters
- **Contribution System**: Allow users to contribute Bitcoin to pools with minimum and maximum limits
- **Proposal Submission**: Enable users to submit proposals with Bitcoin addresses and descriptions
- **Voting Mechanism**: Implement a fair voting system with thresholds and quorum requirements
- **Fund Distribution**: Automatically determine winning proposals and execute Bitcoin transfers
- **Emergency Withdrawal**: Allow contributors to withdraw funds before voting begins

## Architecture

The system is built on the Arch Network framework and consists of the following components:

### Core Modules

- **Contract**: The main contract implementation that manages the pool state and operations
- **PoolParams**: Configuration parameters for the pool
- **Proposal**: Structure for storing proposal information
- **PoolState**: Enum representing the different states of the pool lifecycle

### Integration with Arch Program

The contract integrates with the Arch Program framework, utilizing:

- **AccountInfo**: For account management
- **Pubkey**: For user identification
- **ProgramError**: For error handling
- **Transaction**: For Bitcoin transaction creation
- **Borsh Serialization**: For data serialization/deserialization

## Import Structure

The contract uses the following import structure to integrate with the Arch Network framework:

```rust
use arch_program::{
    account::AccountInfo,
    bitcoin::{self, absolute::LockTime, transaction::Version, Transaction},
    entrypoint,
    helper::add_state_transition,
    input_to_sign::InputToSign,
    msg,
    program::{
        get_account_script_pubkey, get_bitcoin_block_height, next_account_info,
        set_transaction_to_sign,
    },
    program_error::ProgramError,
    pubkey::Pubkey,
    transaction_to_sign::TransactionToSign,
};
use borsh::{BorshDeserialize, BorshSerialize};
```

This structure shows the exact dependencies required for the contract to function properly within the Arch Network ecosystem. The contract relies on:

1. **Account Management**: `AccountInfo` for handling account data
2. **Bitcoin Integration**: `LockTime`, `Version`, and `Transaction` for Bitcoin transaction creation
3. **Program Flow**: `entrypoint` and `add_state_transition` for program execution
4. **Transaction Signing**: `InputToSign` and `TransactionToSign` for preparing transactions
5. **Utility Functions**: Various helper functions for Bitcoin operations
6. **Error Handling**: `ProgramError` for standardized error management
7. **Serialization**: `BorshDeserialize` and `BorshSerialize` for data encoding/decoding

## Installation

### Prerequisites

- Rust 1.60.0 or higher
- Cargo package manager
- Arch Network SDK

### Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/your-username/arch-network-contract.git
   cd arch-network-contract
   ```

2. Build the project:
   ```bash
   cargo build
   ```

3. Run tests:
   ```bash
   cargo test
   ```

## Usage

### Initializing a Pool

```rust
// Create pool parameters
let params = PoolParams {
    min_contribution: 1000,
    max_contribution: 10000,
    contribution_deadline: now + 86400, // 1 day from now
    voting_deadline: now + 172800,      // 2 days from now
    proposal_threshold: 2000,
    voting_threshold: 1000,
    quorum_percentage: 60,
};

// Initialize the pool
contract.initialize_pool(params)?;
```

### Contributing to a Pool

```rust
// Contribute to the pool
contract.contribute(contributor_pubkey, 5000)?;
```

### Submitting a Proposal

```rust
// Submit a proposal
let proposal_id = contract.submit_proposal(
    proposer_pubkey,
    "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_string(),
    "Fund Bitcoin Core development".to_string()
)?;
```

### Casting a Vote

```rust
// Cast a vote for a proposal
contract.cast_vote(voter_pubkey, proposal_id)?;
```

### Executing a Transfer

```rust
// Execute transfer to winning proposal
contract.execute_transfer(program_id, accounts)?;
```

### Emergency Withdrawal

```rust
// Perform emergency withdrawal
let amount = contract.emergency_withdraw(contributor_pubkey)?;
```

## Contract Lifecycle

The contract goes through the following phases:

1. **Uninitialized**: Initial state before pool creation
2. **ContributionPhase**: Users can contribute Bitcoin to the pool
3. **VotingPhase**: Users can submit proposals and vote
4. **ExecutionPhase**: The winning proposal is determined and funds are transferred
5. **Completed**: The contract has completed its lifecycle

Phase transitions occur automatically based on timestamps:
- ContributionPhase → VotingPhase: When contribution_deadline is reached
- VotingPhase → ExecutionPhase: When voting_deadline is reached

## API Reference

### Contract Methods

| Method | Description | Parameters | Returns |
|--------|-------------|------------|---------|
| `initialize_pool` | Creates a new pool | `params: PoolParams` | `Result<(), ContractError>` |
| `contribute` | Adds funds to the pool | `contributor: Pubkey, amount: u64` | `Result<(), ContractError>` |
| `submit_proposal` | Creates a new proposal | `proposer: Pubkey, bitcoin_address: String, description: String` | `Result<u64, ContractError>` |
| `cast_vote` | Votes for a proposal | `voter: Pubkey, proposal_id: u64` | `Result<(), ContractError>` |
| `execute_transfer` | Transfers funds to winning proposal | `program_id: &Pubkey, accounts: &[AccountInfo]` | `Result<(), ContractError>` |
| `emergency_withdraw` | Withdraws funds before voting | `contributor: Pubkey` | `Result<u64, ContractError>` |
| `get_pool_info` | Gets pool information | | `Result<PoolInfo, ContractError>` |
| `get_proposals` | Gets all proposals | | `Vec<Proposal>` |
| `get_winning_proposal` | Gets the winning proposal | | `Option<Proposal>` |

### Instruction Processing

| Instruction | Description | Parameters |
|-------------|-------------|------------|
| `InitializePool` | Creates a new pool | `params: PoolParams` |
| `Contribute` | Adds funds to the pool | `amount: u64` |
| `SubmitProposal` | Creates a new proposal | `bitcoin_address: String, description: String` |
| `CastVote` | Votes for a proposal | `proposal_id: u64` |
| `ExecuteTransfer` | Transfers funds to winning proposal | |
| `EmergencyWithdraw` | Withdraws funds before voting | |

## Testing

The project includes comprehensive tests for all contract functionality:

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_initialize_pool

# Run tests with output
cargo test -- --nocapture
```
## 🧪 Test Coverage

![Test Coverage](https://media-hosting.imagekit.io/712186d02ea148be/Screenshot%202025-04-17%20223740.png?Expires=1839569983&Key-Pair-Id=K2ZIVPTIP2VGHC&Signature=TRyK-W7RHCMk7HcZyDypB7BuB0BTr5HUsHDY16lYuJa7CpacALXxPBt~Gkye0P87P2KKsALKYXCFYdye5FDmZKLtBQrMgmooBitWyFJ-7A8lBigper48mc~hpK7IF-mYEp7HSLc8VkrZNN5KOJY9XJgsP~KUpr~9yjO42Djm3I3j7i~Q53xvwEkWVRDDhXfEpOyIZfACIgwbLWy2lq6UVSAhqrYsuUt8-mJoyM6-LrIVsP0NCaLHRGiaNsiE7zXfosAdBQUVitoIgdzNAFcdQriFQaNQt-fyzhofUCDelnivPCbhcop640GAaE53-4mW~9amxyVFPxhNtEtM7yPdoA__)

The smart contract was tested for:

- Pool initialization  
- Contribution validation  
- Proposal submission  
- Voting mechanism  
- Transfer execution  
- Emergency withdrawal  
- Error handling


## Security Considerations

- **Contribution Limits**: The contract enforces minimum and maximum contribution limits
- **Proposal Threshold**: Only users who have contributed above a threshold can submit proposals
- **Voting Threshold**: Only users who have contributed above a threshold can vote
- **Quorum Requirement**: A minimum percentage of contributors must vote for a valid decision
- **Timelock**: The contract enforces deadlines for contributions and voting
- **Emergency Withdrawal**: Contributors can withdraw funds before voting begins

## Contributing

Contributions are welcome! Please follow these steps:

1. Fork the repository
2. Create a feature branch: `git checkout -b feature-name`
3. Commit your changes: `git commit -m 'Add some feature'`
4. Push to the branch: `git push origin feature-name`
5. Submit a pull request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

---

## Acknowledgments

- Arch Network Team for providing the framework
- Bitcoin Core developers for inspiration
- The Rust community for excellent tools and libraries
