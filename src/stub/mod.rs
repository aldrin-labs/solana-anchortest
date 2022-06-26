use anchor_lang::prelude::*;
use anchor_lang::solana_program::entrypoint::ProgramResult;
use anchor_lang::solana_program::instruction::Instruction;
use std::sync::{Arc, Mutex};

pub trait ValidateCpis {
    fn validate_next_instruction(
        &mut self,
        ix: &Instruction,
        accounts: &[AccountInfo],
    );
}

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
