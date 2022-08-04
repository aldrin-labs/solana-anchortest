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
use std::{mem, slice};

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
///
/// Can be cloned, as all its inner workings are behind sync primitives.
#[derive(Default, Clone, Debug)]
pub struct Syscalls<T> {
    cpi_validator: Arc<Mutex<T>>,
    clock: Arc<Mutex<Clock>>,
    // All captured solana logs are pushed into this vector in order
    logs: Arc<Mutex<Vec<String>>>,
}

impl<T: ValidateCpis + Send + Sync + 'static> Syscalls<T> {
    pub fn new(cpi_validator: T) -> Self {
        Self {
            cpi_validator: Arc::new(Mutex::new(cpi_validator)),
            logs: Default::default(),
            clock: Default::default(),
        }
    }

    /// Returns all logs captured so far.
    pub fn logs(&self) -> Vec<String> {
        self.logs.lock().unwrap().clone()
    }

    /// Sets the slot returned by solana sysvar. This mutates the slot on the
    /// clock object stored on this struct.
    ///
    /// This method has no effect without calling [`Syscalls::set`]
    pub fn slot(&self, slot: u64) {
        let mut guard = self.clock.lock().unwrap();
        guard.slot = slot;
    }

    /// Overwrites the clock object.
    ///
    /// This method has no effect without calling [`Syscalls::set`]
    pub fn clock(&self, clock: Clock) {
        let mut guard = self.clock.lock().unwrap();
        *guard = clock;
    }

    pub fn validator(&self) -> Arc<Mutex<T>> {
        Arc::clone(&self.cpi_validator)
    }

    pub fn set(self) {
        solana_sdk::program_stubs::set_syscall_stubs(Box::new(self));
    }
}

impl<T: ValidateCpis + Send + Sync> solana_sdk::program_stubs::SyscallStubs
    for Syscalls<T>
{
    fn sol_log(&self, message: &str) {
        self.logs.lock().unwrap().push(message.to_string());
        println!("[LOG] {}", message);
    }

    fn sol_get_clock_sysvar(&self, var_addr: *mut u8) -> u64 {
        let size_of_clock = mem::size_of::<Clock>();
        let clock = &*self.clock.lock().unwrap();
        unsafe {
            let var = slice::from_raw_parts_mut(var_addr, size_of_clock);
            let clock_bytes = slice::from_raw_parts(
                (clock as *const Clock) as *const u8,
                size_of_clock,
            );
            var.copy_from_slice(clock_bytes);
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    struct StubValidator;
    impl ValidateCpis for StubValidator {
        fn validate_next_instruction(
            &mut self,
            _ix: &Instruction,
            _accounts: &[AccountInfo],
        ) {
            unimplemented!()
        }
    }

    #[test]
    fn it_sets_clock() {
        let syscalls = Syscalls::new(StubValidator);
        syscalls.clock(Clock {
            slot: 1,
            unix_timestamp: 2,
            ..Default::default()
        });
        syscalls.set();
        assert_eq!(
            Clock::get().unwrap(),
            Clock {
                slot: 1,
                unix_timestamp: 2,
                ..Default::default()
            }
        );

        let syscalls = Syscalls::new(StubValidator);
        syscalls.slot(10);
        syscalls.set();
        assert_eq!(
            Clock::get().unwrap(),
            Clock {
                slot: 10,
                ..Default::default()
            }
        );
    }
}
