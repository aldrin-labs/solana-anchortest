//! Some helper methods for creating common account data structures.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::program_option::COption;
pub use anchor_spl::token::spl_token::state::{Account, AccountState, Mint};

pub trait TokenAccountExt {
    fn amount(self, amount: u64) -> Self;
    fn mint(self, mint: Pubkey) -> Self;
}

pub trait MintExt {
    fn supply(self, supply: u64) -> Self;
}

pub fn token_account(owner: Pubkey) -> Account {
    Account {
        owner,
        mint: Pubkey::new_unique(),
        state: AccountState::Initialized,
        ..Default::default()
    }
}

pub fn mint(mint_authority: Pubkey) -> Mint {
    Mint {
        mint_authority: COption::Some(mint_authority),
        freeze_authority: COption::None,
        is_initialized: true,
        ..Default::default()
    }
}

impl TokenAccountExt for Account {
    fn amount(mut self, amount: u64) -> Self {
        self.amount = amount;
        self
    }

    fn mint(mut self, mint: Pubkey) -> Self {
        self.mint = mint;
        self
    }
}

impl MintExt for Mint {
    fn supply(mut self, supply: u64) -> Self {
        self.supply = supply;
        self
    }
}
