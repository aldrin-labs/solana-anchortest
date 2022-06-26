//! Enables stubbing of CPI calls. For example, you can implement the
//! [`ValidateCpis`] trait for an enum to create a state machine which walks
//! through the enum's variants representing the different CPIs.
//!
//! # Example
//! ```rust,ignore
//! use anchortest::stub::Syscalls;
//! use anchortest::stub::ValidateCpis;
//! use anchor_lang::prelude::AccountInfo;
//! use solana_sdk::instruction::Instruction;
//!
//! struct CpiValidator(CpiValidatorState);
//! enum CpiValidatorState {
//!     FirstCpiCall,
//!     Done,
//! }
//!
//! impl ValidateCpis for CpiValidator {
//!     fn validate_next_instruction(
//!         &mut self,
//!         ix: &Instruction,
//!         _accounts: &[AccountInfo],
//!     ) {
//!         match self.0 {
//!             CpiValidatorState::FirstCpiCall => {
//!                 // TODO: validate
//!
//!                 self.0 = CpiValidatorState::Done;
//!             }
//!             CpiValidatorState::Done => {
//!                 panic!("No more instructions expected, got {:#?}", ix);
//!             }
//!         }
//!     }
//! }
//!
//! Syscalls::new(CpiValidatorState::FirstCpiCall).set();
//! ```

use anchor_lang::prelude::*;
use anchor_lang::solana_program::entrypoint::ProgramResult;
use anchor_lang::solana_program::instruction::Instruction;
use std::sync::{Arc, Mutex};

pub trait ValidateCpis {
    /// Every time the program triggers a CPI, this method is called with the
    /// payload.
    fn validate_next_instruction(
        &mut self,
        ix: &Instruction,
        accounts: &[AccountInfo],
    );
}

/// Holds the necessary state which determines the configurable behavior of
/// syscalls.
#[derive(Default)]
pub struct Syscalls<T> {
    cpi_validator: Arc<Mutex<T>>,
}

impl<T: ValidateCpis + Send + Sync + 'static> Syscalls<T> {
    pub fn new(cpi_validator: T) -> Self {
        Self {
            cpi_validator: Arc::new(Mutex::new(cpi_validator)),
        }
    }

    pub fn set(self) {
        solana_sdk::program_stubs::set_syscall_stubs(Box::new(self));
    }
}

impl<T: ValidateCpis + Send + Sync> solana_sdk::program_stubs::SyscallStubs
    for Syscalls<T>
{
    fn sol_log(&self, message: &str) {
        println!("[LOG] {}", message);
    }

    fn sol_get_clock_sysvar(&self, _var_addr: *mut u8) -> u64 {
        0
    }

    fn sol_get_epoch_schedule_sysvar(&self, _var_addr: *mut u8) -> u64 {
        0
    }

    fn sol_get_fees_sysvar(&self, _var_addr: *mut u8) -> u64 {
        0
    }

    fn sol_get_rent_sysvar(&self, _var_addr: *mut u8) -> u64 {
        0
    }

    fn sol_invoke_signed(
        &self,
        instruction: &Instruction,
        account_infos: &[AccountInfo<'_>],
        _signers_seeds: &[&[&[u8]]],
    ) -> ProgramResult {
        let mut cpis = self.cpi_validator.lock().expect("Cannot obtain lock");

        cpis.validate_next_instruction(instruction, account_infos);

        Ok(())
    }
}
