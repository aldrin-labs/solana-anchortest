//! Some helper methods for creating common account data structures.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::program_option::COption;
pub use anchor_spl::token::spl_token::state::{
    Account as TokenAccount, AccountState, Mint,
};
use solana_sdk::program_pack::Pack;

pub use mint::MintExt;
pub use token_account::TokenAccountExt;

pub mod mint {
    use super::*;

    pub trait MintExt {
        fn supply(self, supply: u64) -> Self;
    }

    pub fn new(mint_authority: Pubkey) -> Mint {
        Mint {
            mint_authority: COption::Some(mint_authority),
            freeze_authority: COption::None,
            is_initialized: true,
            ..Default::default()
        }
    }

    pub fn from_acc_info(mint: &AccountInfo) -> Mint {
        Mint::unpack_from_slice(&mut mint.data.borrow())
            .expect("Should deserialize mint")
    }

    impl MintExt for Mint {
        fn supply(mut self, supply: u64) -> Self {
            self.supply = supply;
            self
        }
    }

    /// Adds given amount of tokens to the wallet and increases the supply.
    pub fn mint_to(
        wallet: &AccountInfo,
        mint: &AccountInfo,
        amount: u64,
    ) -> Option<()> {
        let mut m = from_acc_info(mint);
        m.supply = m.supply.checked_add(amount)?;

        token_account::change_amount(wallet, amount as i128)?;

        // write the new supply after we successfully change the amount
        Mint::pack_into_slice(&m, &mut mint.data.borrow_mut());

        Some(())
    }

    /// Removes given amount of tokens from the wallet and decreases the supply.
    pub fn burn_from(
        wallet: &AccountInfo,
        mint: &AccountInfo,
        amount: u64,
    ) -> Option<()> {
        let mut m = from_acc_info(mint);
        m.supply = m.supply.checked_sub(amount)?;

        token_account::change_amount(wallet, -(amount as i128))?;

        // write the new supply after we successfully change the amount
        Mint::pack_into_slice(&m, &mut mint.data.borrow_mut());

        Some(())
    }
}

pub mod token_account {
    use super::*;

    pub trait TokenAccountExt {
        fn amount(self, amount: u64) -> Self;
        fn mint(self, mint: Pubkey) -> Self;
    }

    impl TokenAccountExt for TokenAccount {
        fn amount(mut self, amount: u64) -> Self {
            self.amount = amount;
            self
        }

        fn mint(mut self, mint: Pubkey) -> Self {
            self.mint = mint;
            self
        }
    }

    pub fn new(owner: Pubkey) -> TokenAccount {
        TokenAccount {
            owner,
            mint: Pubkey::new_unique(),
            state: AccountState::Initialized,
            ..Default::default()
        }
    }

    pub fn from_acc_info(info: &AccountInfo) -> TokenAccount {
        TokenAccount::unpack_from_slice(&mut info.data.borrow())
            .expect("Should deserialize account")
    }

    /// Use negative number to deduct from amount, positive to add.
    ///
    /// Returns [`Some`] with previous amount on success, or [`None`] on
    /// overflow.
    ///
    /// # Panics
    /// If the account info cannot be unpacked to a [`TokenAccount`].
    pub fn change_amount(info: &AccountInfo, amount: i128) -> Option<u64> {
        let mut wallet = from_acc_info(info);
        let current_amount = wallet.amount;

        if amount >= 0 {
            wallet.amount = wallet.amount.checked_add(amount as u64)?;
        } else {
            wallet.amount = wallet.amount.checked_sub(amount.abs() as u64)?;
        }

        TokenAccount::pack_into_slice(&wallet, &mut info.data.borrow_mut());

        Some(current_amount)
    }

    /// Transfers given amount from the first account into the second.
    ///
    /// Returns [`Some`] on success and [`None`] if not enough tokens are
    /// available in the source account.
    ///
    /// # Panics
    /// If either account cannot be deserialized.
    pub fn transfer(
        from: &AccountInfo,
        into: &AccountInfo,
        amount: u64,
    ) -> Option<()> {
        change_amount(from, -(amount as i128))?;
        change_amount(into, amount as i128)?;
        Some(())
    }
}
