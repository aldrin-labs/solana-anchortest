use anchor_lang::prelude::*;
use anchor_lang::solana_program::program_option::COption;
pub use anchor_spl::token::spl_token::state::{Account, AccountState, Mint};

pub fn token_account(owner: Pubkey, mint: Option<Pubkey>) -> Account {
    Account {
        mint: mint.unwrap_or_else(Pubkey::new_unique),
        owner,
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
