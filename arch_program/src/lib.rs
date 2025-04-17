// Mock implementation of arch_program for development
pub mod account {
    use std::cell::RefCell;
    use std::rc::Rc;
    use crate::pubkey::Pubkey;

    #[derive(Debug)]
    pub struct AccountInfo<'a> {
        pub key: &'a Pubkey,
        pub is_signer: bool,
        pub is_writable: bool,
        pub lamports: Rc<RefCell<u64>>,
        pub data: Rc<RefCell<Vec<u8>>>,
        pub owner: &'a Pubkey,
        pub executable: bool,
        pub rent_epoch: u64,
    }
}

pub mod bitcoin {
    pub mod absolute {
        use thiserror::Error;

        #[derive(Debug, Clone)]
        pub struct LockTime(pub u32);

        impl LockTime {
            pub fn from_height(height: u32) -> Result<Self, LockTimeError> {
                Ok(LockTime(height))
            }
        }

        #[derive(Error, Debug)]
        pub enum LockTimeError {
            #[error("Invalid lock time")]
            InvalidLockTime,
        }
    }

    pub mod transaction {
        #[derive(Debug, Clone)]
        pub enum Version {
            ONE,
            TWO,
        }
    }

    #[derive(Debug, Clone)]
    pub struct Transaction {
        pub version: transaction::Version,
        pub lock_time: absolute::LockTime,
    }
}

pub mod input_to_sign {
    #[derive(Debug, Clone)]
    pub struct InputToSign {
        // Fields would be defined here in a real implementation
    }
}

pub mod transaction_to_sign {
    use crate::bitcoin::Transaction;
    use crate::input_to_sign::InputToSign;

    #[derive(Debug, Clone)]
    pub struct TransactionToSign {
        pub transaction: Transaction,
        pub inputs_to_sign: Vec<InputToSign>,
    }
}

pub mod program_error {
    use thiserror::Error;

    #[derive(Error, Debug, Clone, PartialEq)]
    pub enum ProgramError {
        #[error("Custom error: {0}")]
        Custom(u32),
        
        #[error("Invalid instruction data")]
        InvalidInstructionData,
        
        #[error("Incorrect program ID")]
        IncorrectProgramId,
        
        #[error("Not enough account keys")]
        NotEnoughAccountKeys,
    }
}

pub mod pubkey {
    use std::fmt;
    use std::hash::Hash;
    use borsh::{BorshSerialize, BorshDeserialize};
    use std::io::{Read, Write};

    #[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Pubkey([u8; 32]);

    impl Pubkey {
        pub fn new_unique() -> Self {
            use std::sync::atomic::{AtomicU8, Ordering};
            static COUNTER: AtomicU8 = AtomicU8::new(0);
            let mut key = [0u8; 32];
            key[0] = COUNTER.fetch_add(1, Ordering::Relaxed);
            Pubkey(key)
        }
    }

    impl fmt::Debug for Pubkey {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "Pubkey({:?})", &self.0[..])
        }
    }

    // Implement BorshSerialize for Pubkey
    impl BorshSerialize for Pubkey {
        fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
            writer.write_all(&self.0)
        }
    }

    // Implement BorshDeserialize for Pubkey
    impl BorshDeserialize for Pubkey {
        fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
            if buf.len() < 32 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Not enough bytes for Pubkey",
                ));
            }
            
            let mut array = [0u8; 32];
            array.copy_from_slice(&buf[..32]);
            *buf = &buf[32..];
            
            Ok(Pubkey(array))
        }
        
        fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
            let mut array = [0u8; 32];
            reader.read_exact(&mut array)?;
            Ok(Pubkey(array))
        }
    }
}

pub mod program {
    use crate::account::AccountInfo;
    use crate::program_error::ProgramError;

    pub fn next_account_info<'a, 'b, I: Iterator<Item = &'a AccountInfo<'b>>>(
        iter: &mut I,
    ) -> Result<&'a AccountInfo<'b>, ProgramError> {
        iter.next().ok_or(ProgramError::NotEnoughAccountKeys)
    }

    pub fn get_account_script_pubkey(_address: &str) -> Result<Vec<u8>, ProgramError> {
        // Mock implementation
        Ok(vec![0; 32])
    }

    pub fn get_bitcoin_block_height() -> Result<u32, ProgramError> {
        // Mock implementation
        Ok(100000)
    }

    pub fn set_transaction_to_sign(_transaction: crate::transaction_to_sign::TransactionToSign) -> Result<(), ProgramError> {
        // Mock implementation
        Ok(())
    }
}

pub mod helper {
    use crate::account::AccountInfo;
    use crate::program_error::ProgramError;
    use crate::pubkey::Pubkey;
    use borsh::BorshSerialize;

    pub fn add_state_transition<T: BorshSerialize>(
        _payer: &AccountInfo,
        _program_id: &Pubkey,
        _state: &T,
    ) -> Result<(), ProgramError> {
        // Mock implementation
        Ok(())
    }
}

// Msg macro definition
#[macro_export]
macro_rules! msg {
    ($($arg:tt)*) => {
        println!($($arg)*);
    };
}

// Entrypoint macro definition
#[macro_export]
macro_rules! entrypoint {
    ($process_instruction:ident) => {
        // Mock implementation
        pub fn entrypoint(
            program_id: &$crate::pubkey::Pubkey,
            accounts: &[$crate::account::AccountInfo],
            instruction_data: &[u8],
        ) -> Result<(), $crate::program_error::ProgramError> {
            $process_instruction(program_id, accounts, instruction_data)
        }
    };
}
