use arch_program::bitcoin::absolute;
use arch_program::{
    account::AccountInfo,
    bitcoin::{absolute::LockTime, transaction::Version, Transaction},
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
use std::collections::HashMap;
use chrono::Utc;
use std::io::{Read, Write};

/// Error types for the Arch Network contract
#[derive(Debug, Clone)]
pub enum ContractError {
    PoolNotInitialized,
    PoolAlreadyInitialized,
    ContributionTooLow,
    ContributionTooHigh,
    PoolDeadlinePassed,
    VotingPeriodNotEnded,
    VotingPeriodEnded,
    ContributorNotFound,
    InsufficientContributionForProposal,
    InsufficientContributionForVoting,
    ProposalNotFound,
    AlreadyVoted,
    InvalidBitcoinAddress,
    NoProposalsSubmitted,
    NoVotesCast,
    QuorumNotReached,
    TransferAlreadyExecuted,
    ProgramError(ProgramError),
    LockTimeError,
    IoError(String),
}

impl From<ProgramError> for ContractError {
    fn from(error: ProgramError) -> Self {
        ContractError::ProgramError(error)
    }
}

impl From<bitcoin::absolute::LockTimeError> for ContractError {
    fn from(_: absolute::LockTimeError) -> Self {
        ContractError::LockTimeError
    }
}

impl From<std::io::Error> for ContractError {
    fn from(error: std::io::Error) -> Self {
        ContractError::IoError(error.to_string())
    }
}

impl From<ContractError> for ProgramError {
    fn from(error: ContractError) -> Self {
        match error {
            ContractError::PoolNotInitialized => ProgramError::Custom(1),
            ContractError::PoolAlreadyInitialized => ProgramError::Custom(2),
            ContractError::ContributionTooLow => ProgramError::Custom(3),
            ContractError::ContributionTooHigh => ProgramError::Custom(4),
            ContractError::PoolDeadlinePassed => ProgramError::Custom(5),
            ContractError::VotingPeriodNotEnded => ProgramError::Custom(6),
            ContractError::VotingPeriodEnded => ProgramError::Custom(7),
            ContractError::ContributorNotFound => ProgramError::Custom(8),
            ContractError::InsufficientContributionForProposal => ProgramError::Custom(9),
            ContractError::InsufficientContributionForVoting => ProgramError::Custom(10),
            ContractError::ProposalNotFound => ProgramError::Custom(11),
            ContractError::AlreadyVoted => ProgramError::Custom(12),
            ContractError::InvalidBitcoinAddress => ProgramError::Custom(13),
            ContractError::NoProposalsSubmitted => ProgramError::Custom(14),
            ContractError::NoVotesCast => ProgramError::Custom(15),
            ContractError::QuorumNotReached => ProgramError::Custom(16),
            ContractError::TransferAlreadyExecuted => ProgramError::Custom(17),
            ContractError::ProgramError(e) => e,
            ContractError::LockTimeError => ProgramError::Custom(18),
            ContractError::IoError(_) => ProgramError::Custom(19),
        }
    }
}

/// Pool parameters
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct PoolParams {
    pub min_contribution: u64,
    pub max_contribution: u64,
    pub contribution_deadline: i64, // Unix timestamp
    pub voting_deadline: i64,       // Unix timestamp
    pub proposal_threshold: u64,
    pub voting_threshold: u64,
    pub quorum_percentage: u8,
}

/// Proposal structure
#[derive(Clone, Debug)]
pub struct Proposal {
    pub id: u64,
    pub proposer: Pubkey,
    pub bitcoin_address: String,
    pub description: String,
    pub votes: u64,
}

// Implement BorshSerialize for Proposal
impl BorshSerialize for Proposal {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.id.serialize(writer)?;
        self.proposer.serialize(writer)?;
        self.bitcoin_address.serialize(writer)?;
        self.description.serialize(writer)?;
        self.votes.serialize(writer)?;
        Ok(())
    }
}

// Implement BorshDeserialize for Proposal
impl BorshDeserialize for Proposal {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let id = u64::deserialize_reader(reader)?;
        let proposer = Pubkey::deserialize_reader(reader)?;
        let bitcoin_address = String::deserialize_reader(reader)?;
        let description = String::deserialize_reader(reader)?;
        let votes = u64::deserialize_reader(reader)?;

        Ok(Proposal {
            id,
            proposer,
            bitcoin_address,
            description,
            votes,
        })
    }
}

/// Pool state
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq)]
pub enum PoolState {
    Uninitialized,
    ContributionPhase,
    VotingPhase,
    ExecutionPhase,
    Completed,
}

/// Contract state
#[derive(Clone, Debug)]
pub struct Contract {
    pub state: PoolState,
    pub params: Option<PoolParams>,
    pub total_balance: u64,
    pub contributions: HashMap<Pubkey, u64>,
    pub proposals: HashMap<u64, Proposal>,
    pub votes: HashMap<Pubkey, u64>, // contributor -> proposal_id
    pub next_proposal_id: u64,
    pub winning_proposal: Option<u64>,
    pub transfer_executed: bool,
}

// Custom serialization for HashMap<Pubkey, u64>
fn serialize_pubkey_map<W: Write>(
    map: &HashMap<Pubkey, u64>,
    writer: &mut W,
) -> std::io::Result<()> {
    let len = map.len() as u32;
    len.serialize(writer)?;
    for (key, value) in map.iter() {
        key.serialize(writer)?;
        value.serialize(writer)?;
    }
    Ok(())
}

// Custom deserialization for HashMap<Pubkey, u64>
fn deserialize_pubkey_map(buf: &mut &[u8]) -> std::io::Result<HashMap<Pubkey, u64>> {
    let len = u32::deserialize(buf)?;
    let mut map = HashMap::new();
    for _ in 0..len {
        let key = Pubkey::deserialize(buf)?;
        let value = u64::deserialize(buf)?;
        map.insert(key, value);
    }
    Ok(map)
}

// Custom serialization for HashMap<u64, Proposal>
fn serialize_proposal_map<W: Write>(
    map: &HashMap<u64, Proposal>,
    writer: &mut W,
) -> std::io::Result<()> {
    let len = map.len() as u32;
    len.serialize(writer)?;
    for (key, value) in map.iter() {
        key.serialize(writer)?;
        value.serialize(writer)?;
    }
    Ok(())
}

// Custom deserialization for HashMap<u64, Proposal>
fn deserialize_proposal_map(buf: &mut &[u8]) -> std::io::Result<HashMap<u64, Proposal>> {
    let len = u32::deserialize(buf)?;
    let mut map = HashMap::new();
    for _ in 0..len {
        let key = u64::deserialize(buf)?;
        let value = Proposal::deserialize(buf)?;
        map.insert(key, value);
    }
    Ok(map)
}

// Implement BorshSerialize for Contract
impl BorshSerialize for Contract {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.state.serialize(writer)?;
        
        // Serialize Option<PoolParams>
        match &self.params {
            Some(params) => {
                1u8.serialize(writer)?; // Some variant
                params.serialize(writer)?;
            }
            None => {
                0u8.serialize(writer)?; // None variant
            }
        }
        
        self.total_balance.serialize(writer)?;
        
        // Serialize HashMap<Pubkey, u64>
        serialize_pubkey_map(&self.contributions, writer)?;
        
        // Serialize HashMap<u64, Proposal>
        serialize_proposal_map(&self.proposals, writer)?;
        
        // Serialize HashMap<Pubkey, u64>
        serialize_pubkey_map(&self.votes, writer)?;
        
        self.next_proposal_id.serialize(writer)?;
        
        // Serialize Option<u64>
        match self.winning_proposal {
            Some(id) => {
                1u8.serialize(writer)?; // Some variant
                id.serialize(writer)?;
            }
            None => {
                0u8.serialize(writer)?; // None variant
            }
        }
        
        self.transfer_executed.serialize(writer)?;
        
        Ok(())
    }
}

// Implement BorshDeserialize for Contract
impl BorshDeserialize for Contract {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let state = PoolState::deserialize(buf)?;
        
        // Deserialize Option<PoolParams>
        let params = match u8::deserialize(buf)? {
            0 => None,
            1 => Some(PoolParams::deserialize(buf)?),
            _ => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid option tag")),
        };
        
        let total_balance = u64::deserialize(buf)?;
        
        // Deserialize HashMap<Pubkey, u64>
        let contributions = deserialize_pubkey_map(buf)?;
        
        // Deserialize HashMap<u64, Proposal>
        let proposals = deserialize_proposal_map(buf)?;
        
        // Deserialize HashMap<Pubkey, u64>
        let votes = deserialize_pubkey_map(buf)?;
        
        let next_proposal_id = u64::deserialize(buf)?;
        
        // Deserialize Option<u64>
        let winning_proposal = match u8::deserialize(buf)? {
            0 => None,
            1 => Some(u64::deserialize(buf)?),
            _ => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid option tag")),
        };
        
        let transfer_executed = bool::deserialize(buf)?;
        
        Ok(Contract {
            state,
            params,
            total_balance,
            contributions,
            proposals,
            votes,
            next_proposal_id,
            winning_proposal,
            transfer_executed,
        })
    }

    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;
        let mut slice = buf.as_slice();
        Self::deserialize(&mut slice)
    }
}

impl Default for Contract {
    fn default() -> Self {
        Self {
            state: PoolState::Uninitialized,
            params: None,
            total_balance: 0,
            contributions: HashMap::new(),
            proposals: HashMap::new(),
            votes: HashMap::new(),
            next_proposal_id: 1,
            winning_proposal: None,
            transfer_executed: false,
        }
    }
}

impl Contract {
    /// Initialize a new pool with the given parameters
    pub fn initialize_pool(&mut self, params: PoolParams) -> Result<(), ContractError> {
        if self.state != PoolState::Uninitialized {
            return Err(ContractError::PoolAlreadyInitialized);
        }
        
        // Validate parameters
        if params.min_contribution >= params.max_contribution {
            return Err(ContractError::ContributionTooLow);
        }
        
        if params.contribution_deadline >= params.voting_deadline {
            return Err(ContractError::PoolDeadlinePassed);
        }
        
        if params.quorum_percentage > 100 {
            return Err(ContractError::QuorumNotReached);
        }
        
        self.params = Some(params);
        self.state = PoolState::ContributionPhase;
        
        Ok(())
    }
    
    /// Contribute to the pool
    pub fn contribute(&mut self, contributor: Pubkey, amount: u64) -> Result<(), ContractError> {
        let params = self.params.as_ref().ok_or(ContractError::PoolNotInitialized)?;
        
        if self.state != PoolState::ContributionPhase {
            return Err(ContractError::PoolDeadlinePassed);
        }
        
        let now = Utc::now().timestamp();
        if now > params.contribution_deadline {
            self.state = PoolState::VotingPhase;
            return Err(ContractError::PoolDeadlinePassed);
        }
        
        if amount < params.min_contribution {
            return Err(ContractError::ContributionTooLow);
        }
        
        if amount > params.max_contribution {
            return Err(ContractError::ContributionTooHigh);
        }
        
        // Update or add contribution
        let current_contribution = self.contributions.get(&contributor).unwrap_or(&0);
        let new_total = current_contribution + amount;
        
        if new_total > params.max_contribution {
            return Err(ContractError::ContributionTooHigh);
        }
        
        self.contributions.insert(contributor, new_total);
        self.total_balance += amount;
        
        Ok(())
    }
    
    /// Submit a proposal
    pub fn submit_proposal(
        &mut self,
        proposer: Pubkey,
        bitcoin_address: String,
        description: String,
    ) -> Result<u64, ContractError> {
        let params = self.params.as_ref().ok_or(ContractError::PoolNotInitialized)?;
        
        if self.state != PoolState::VotingPhase {
            let now = Utc::now().timestamp();
            if now > params.contribution_deadline {
                self.state = PoolState::VotingPhase;
            } else {
                return Err(ContractError::PoolDeadlinePassed);
            }
        }
        
        let now = Utc::now().timestamp();
        if now > params.voting_deadline {
            self.state = PoolState::ExecutionPhase;
            return Err(ContractError::VotingPeriodEnded);
        }
        
        // Check if proposer has contributed enough
        let contribution = self.contributions.get(&proposer).unwrap_or(&0);
        if *contribution < params.proposal_threshold {
            return Err(ContractError::InsufficientContributionForProposal);
        }
        
        // Validate Bitcoin address (simple validation)
        if !is_valid_bitcoin_address(&bitcoin_address) {
            return Err(ContractError::InvalidBitcoinAddress);
        }
        
        // Create and store proposal
        let proposal_id = self.next_proposal_id;
        self.next_proposal_id += 1;
        
        let proposal = Proposal {
            id: proposal_id,
            proposer,
            bitcoin_address,
            description,
            votes: 0,
        };
        
        self.proposals.insert(proposal_id, proposal);
        
        Ok(proposal_id)
    }
    
    /// Cast a vote for a proposal
    pub fn cast_vote(&mut self, voter: Pubkey, proposal_id: u64) -> Result<(), ContractError> {
        let params = self.params.as_ref().ok_or(ContractError::PoolNotInitialized)?;
        
        if self.state != PoolState::VotingPhase {
            let now = Utc::now().timestamp();
            if now > params.contribution_deadline && now <= params.voting_deadline {
                self.state = PoolState::VotingPhase;
            } else if now > params.voting_deadline {
                self.state = PoolState::ExecutionPhase;
                return Err(ContractError::VotingPeriodEnded);
            } else {
                return Err(ContractError::PoolDeadlinePassed);
            }
        }
        
        // Check if voter has contributed enough
        let contribution = self.contributions.get(&voter).unwrap_or(&0);
        if *contribution < params.voting_threshold {
            return Err(ContractError::InsufficientContributionForVoting);
        }
        
        // Check if proposal exists
        if !self.proposals.contains_key(&proposal_id) {
            return Err(ContractError::ProposalNotFound);
        }
        
        // Check if already voted
        if self.votes.contains_key(&voter) {
            return Err(ContractError::AlreadyVoted);
        }
        
        // Record vote
        self.votes.insert(voter, proposal_id);
        
        // Update proposal vote count
        if let Some(proposal) = self.proposals.get_mut(&proposal_id) {
            proposal.votes += 1;
        }
        
        Ok(())
    }
    
    /// Execute transfer to the winning proposal
    pub fn execute_transfer(&mut self, program_id: &Pubkey, accounts: &[AccountInfo]) -> Result<(), ContractError> {
        let params = self.params.as_ref().ok_or(ContractError::PoolNotInitialized)?;
        
        if self.state != PoolState::ExecutionPhase {
            let now = Utc::now().timestamp();
            if now <= params.voting_deadline {
                return Err(ContractError::VotingPeriodNotEnded);
            } else {
                self.state = PoolState::ExecutionPhase;
            }
        }
        
        if self.transfer_executed {
            return Err(ContractError::TransferAlreadyExecuted);
        }
        
        if self.proposals.is_empty() {
            return Err(ContractError::NoProposalsSubmitted);
        }
        
        if self.votes.is_empty() {
            return Err(ContractError::NoVotesCast);
        }
        
        // Check quorum
        let total_contributors = self.contributions.len() as f64;
        let total_voters = self.votes.len() as f64;
        let quorum_percentage = params.quorum_percentage as f64 / 100.0;
        
        if (total_voters / total_contributors) < quorum_percentage {
            return Err(ContractError::QuorumNotReached);
        }
        
        // Find winning proposal
        let mut winning_proposal_id = 0;
        let mut max_votes = 0;
        
        for (id, proposal) in &self.proposals {
            if proposal.votes > max_votes {
                max_votes = proposal.votes;
                winning_proposal_id = *id;
            }
        }
        
        if winning_proposal_id == 0 {
            return Err(ContractError::NoVotesCast);
        }
        
        // Get winning proposal
        let winning_proposal = self.proposals.get(&winning_proposal_id)
            .ok_or(ContractError::ProposalNotFound)?;
        
        // Create Bitcoin transaction
        let account_info_iter = &mut accounts.iter();
        let payer = next_account_info(account_info_iter)?;
        
        // Get Bitcoin script pubkey from address
        let script_pubkey = get_account_script_pubkey(winning_proposal.bitcoin_address.as_str())?;
        
        // Create transaction
        let block_height = get_bitcoin_block_height()?;
        let lock_time = LockTime::from_height(block_height)?;
        
        // Prepare transaction to sign
        let transaction = Transaction {
            version: Version::TWO,
            lock_time,
            // Other transaction details would be filled here
            // This is simplified for the example
        };
        
        // Set transaction to sign
        let transaction_to_sign = TransactionToSign {
            transaction,
            inputs_to_sign: vec![
                InputToSign {
                    // Input details would be filled here
                    // This is simplified for the example
                }
            ],
        };
        
        set_transaction_to_sign(transaction_to_sign)?;
        
        // Mark as executed
        self.winning_proposal = Some(winning_proposal_id);
        self.transfer_executed = true;
        self.state = PoolState::Completed;
        
        // Add state transition
        add_state_transition(payer, program_id, self)?;
        
        Ok(())
    }
    
    /// Emergency withdraw before voting deadline
    pub fn emergency_withdraw(&mut self, contributor: Pubkey) -> Result<u64, ContractError> {
        let _params = self.params.as_ref().ok_or(ContractError::PoolNotInitialized)?;
        
        // Only allow withdrawals before voting begins
        if self.state != PoolState::ContributionPhase {
            return Err(ContractError::PoolDeadlinePassed);
        }
        
        // Get contribution
        let contribution = self.contributions.get(&contributor).ok_or(ContractError::ContributorNotFound)?;
        let amount = *contribution;
        
        // Remove contribution
        self.contributions.remove(&contributor);
        self.total_balance -= amount;
        
        Ok(amount)
    }
    
    /// Get pool information
    pub fn get_pool_info(&self) -> Result<PoolInfo, ContractError> {
        let params = self.params.as_ref().ok_or(ContractError::PoolNotInitialized)?;
        
        Ok(PoolInfo {
            state: self.state.clone(),
            total_balance: self.total_balance,
            total_contributors: self.contributions.len() as u64,
            total_proposals: self.proposals.len() as u64,
            total_votes: self.votes.len() as u64,
            contribution_deadline: params.contribution_deadline,
            voting_deadline: params.voting_deadline,
        })
    }
    
    /// Get all proposals
    pub fn get_proposals(&self) -> Vec<Proposal> {
        self.proposals.values().cloned().collect()
    }
    
    /// Get winning proposal
    pub fn get_winning_proposal(&self) -> Option<Proposal> {
        self.winning_proposal.and_then(|id| self.proposals.get(&id).cloned())
    }
}

/// Pool information
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct PoolInfo {
    pub state: PoolState,
    pub total_balance: u64,
    pub total_contributors: u64,
    pub total_proposals: u64,
    pub total_votes: u64,
    pub contribution_deadline: i64, // Unix timestamp
    pub voting_deadline: i64,       // Unix timestamp
}

/// Validate Bitcoin address (simplified)
fn is_valid_bitcoin_address(address: &str) -> bool {
    // This is a simplified validation
    // In a real implementation, this would check the address format and checksum
    address.starts_with("1") || address.starts_with("3") || address.starts_with("bc1")
}

// Entrypoint for the program
entrypoint!(process_instruction);

// Process instruction
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // Deserialize instruction data
    let instruction = ContractInstruction::try_from_slice(instruction_data)
        .map_err(|_| {
            msg!("Failed to deserialize instruction data");
            ProgramError::InvalidInstructionData
        })?;

    // Process instruction based on type
    match instruction {
        ContractInstruction::InitializePool { params } => {
            msg!("Instruction: InitializePool");
            process_initialize_pool(program_id, accounts, params)
        }
        ContractInstruction::Contribute { amount } => {
            msg!("Instruction: Contribute");
            process_contribute(program_id, accounts, amount)
        }
        ContractInstruction::SubmitProposal { bitcoin_address, description } => {
            msg!("Instruction: SubmitProposal");
            process_submit_proposal(program_id, accounts, bitcoin_address, description)
        }
        ContractInstruction::CastVote { proposal_id } => {
            msg!("Instruction: CastVote");
            process_cast_vote(program_id, accounts, proposal_id)
        }
        ContractInstruction::ExecuteTransfer => {
            msg!("Instruction: ExecuteTransfer");
            process_execute_transfer(program_id, accounts)
        }
        ContractInstruction::EmergencyWithdraw => {
            msg!("Instruction: EmergencyWithdraw");
            process_emergency_withdraw(program_id, accounts)
        }
    }
}

// Contract instructions
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum ContractInstruction {
    InitializePool { params: PoolParams },
    Contribute { amount: u64 },
    SubmitProposal { bitcoin_address: String, description: String },
    CastVote { proposal_id: u64 },
    ExecuteTransfer,
    EmergencyWithdraw,
}

// Process initialize pool instruction
fn process_initialize_pool(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    params: PoolParams,
) -> Result<(), ProgramError> {
    let account_info_iter = &mut accounts.iter();
    let contract_account = next_account_info(account_info_iter)?;
    let payer = next_account_info(account_info_iter)?;

    // Check if the contract account is owned by the program
    if contract_account.owner != program_id {
        msg!("Contract account not owned by program");
        return Err(ProgramError::IncorrectProgramId);
    }

    // Deserialize contract state or create new if empty
    let mut contract_data = contract_account.data.borrow();
    let mut contract = if contract_data.len() > 0 {
        match Contract::try_from_slice(&contract_data) {
            Ok(contract) => contract,
            Err(_) => {
                msg!("Failed to deserialize contract state");
                Contract::default()
            }
        }
    } else {
        Contract::default()
    };
    drop(contract_data);

    // Initialize pool
    contract.initialize_pool(params).map_err(|e| e.into())?;

    // Serialize and save contract state
    add_state_transition(payer, program_id, &contract)?;

    Ok(())
}

// Process contribute instruction
fn process_contribute(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> Result<(), ProgramError> {
    let account_info_iter = &mut accounts.iter();
    let contract_account = next_account_info(account_info_iter)?;
    let contributor = next_account_info(account_info_iter)?;
    let payer = next_account_info(account_info_iter)?;

    // Check if the contract account is owned by the program
    if contract_account.owner != program_id {
        msg!("Contract account not owned by program");
        return Err(ProgramError::IncorrectProgramId);
    }

    // Deserialize contract state
    let mut contract_data = contract_account.data.borrow();
    let mut contract = match Contract::try_from_slice(&contract_data) {
        Ok(contract) => contract,
        Err(_) => {
            msg!("Failed to deserialize contract state");
            return Err(ProgramError::InvalidInstructionData);
        }
    };
    drop(contract_data);

    // Contribute
    contract.contribute(*contributor.key, amount).map_err(|e| e.into())?;

    // Serialize and save contract state
    add_state_transition(payer, program_id, &contract)?;

    Ok(())
}

// Process submit proposal instruction
fn process_submit_proposal(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    bitcoin_address: String,
    description: String,
) -> Result<(), ProgramError> {
    let account_info_iter = &mut accounts.iter();
    let contract_account = next_account_info(account_info_iter)?;
    let proposer = next_account_info(account_info_iter)?;
    let payer = next_account_info(account_info_iter)?;

    // Check if the contract account is owned by the program
    if contract_account.owner != program_id {
        msg!("Contract account not owned by program");
        return Err(ProgramError::IncorrectProgramId);
    }

    // Deserialize contract state
    let mut contract_data = contract_account.data.borrow();
    let mut contract = match Contract::try_from_slice(&contract_data) {
        Ok(contract) => contract,
        Err(_) => {
            msg!("Failed to deserialize contract state");
            return Err(ProgramError::InvalidInstructionData);
        }
    };
    drop(contract_data);

    // Submit proposal
    let proposal_id = contract.submit_proposal(*proposer.key, bitcoin_address, description)
        .map_err(|e| e.into())?;

    msg!("Proposal submitted with ID: {}", proposal_id);

    // Serialize and save contract state
    add_state_transition(payer, program_id, &contract)?;

    Ok(())
}

// Process cast vote instruction
fn process_cast_vote(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    proposal_id: u64,
) -> Result<(), ProgramError> {
    let account_info_iter = &mut accounts.iter();
    let contract_account = next_account_info(account_info_iter)?;
    let voter = next_account_info(account_info_iter)?;
    let payer = next_account_info(account_info_iter)?;

    // Check if the contract account is owned by the program
    if contract_account.owner != program_id {
        msg!("Contract account not owned by program");
        return Err(ProgramError::IncorrectProgramId);
    }

    // Deserialize contract state
    let mut contract_data = contract_account.data.borrow();
    let mut contract = match Contract::try_from_slice(&contract_data) {
        Ok(contract) => contract,
        Err(_) => {
            msg!("Failed to deserialize contract state");
            return Err(ProgramError::InvalidInstructionData);
        }
    };
    drop(contract_data);

    // Cast vote
    contract.cast_vote(*voter.key, proposal_id).map_err(|e| e.into())?;

    // Serialize and save contract state
    add_state_transition(payer, program_id, &contract)?;

    Ok(())
}

// Process execute transfer instruction
fn process_execute_transfer(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> Result<(), ProgramError> {
    let account_info_iter = &mut accounts.iter();
    let contract_account = next_account_info(account_info_iter)?;
    
    // Check if the contract account is owned by the program
    if contract_account.owner != program_id {
        msg!("Contract account not owned by program");
        return Err(ProgramError::IncorrectProgramId);
    }

    // Deserialize contract state
    let mut contract_data = contract_account.data.borrow();
    let mut contract = match Contract::try_from_slice(&contract_data) {
        Ok(contract) => contract,
        Err(_) => {
            msg!("Failed to deserialize contract state");
            return Err(ProgramError::InvalidInstructionData);
        }
    };
    drop(contract_data);

    // Execute transfer
    contract.execute_transfer(program_id, accounts).map_err(|e| e.into())?;

    msg!("Transfer executed successfully");

    Ok(())
}

// Process emergency withdraw instruction
fn process_emergency_withdraw(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> Result<(), ProgramError> {
    let account_info_iter = &mut accounts.iter();
    let contract_account = next_account_info(account_info_iter)?;
    let contributor = next_account_info(account_info_iter)?;
    let payer = next_account_info(account_info_iter)?;

    // Check if the contract account is owned by the program
    if contract_account.owner != program_id {
        msg!("Contract account not owned by program");
        return Err(ProgramError::IncorrectProgramId);
    }

    // Deserialize contract state
    let mut contract_data = contract_account.data.borrow();
    let mut contract = match Contract::try_from_slice(&contract_data) {
        Ok(contract) => contract,
        Err(_) => {
            msg!("Failed to deserialize contract state");
            return Err(ProgramError::InvalidInstructionData);
        }
    };
    drop(contract_data);

    // Emergency withdraw
    let amount = contract.emergency_withdraw(*contributor.key).map_err(|e| e.into())?;

    msg!("Emergency withdrawal of {} satoshis successful", amount);

    // Serialize and save contract state
    add_state_transition(payer, program_id, &contract)?;

    Ok(())
}
