//! Helpers to build state which can be used to construct [`Context`]. This then
//! serves as an input to a program's endpoints.

use anchor_lang::solana_program::bpf_loader_upgradeable;
use anchor_lang::{prelude::*, system_program};
use solana_sdk::program_pack::Pack;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

pub type Bumps = BTreeMap<String, u8>;

/// Builder pattern for specifying accounts relevant for a tested endpoint.
#[derive(Default)]
pub struct ContextWrapper<'info> {
    pub program: Pubkey,
    pub accounts: Vec<AccountInfo<'info>>,
    /// Will be accessible via the [`Context`]'s `remaining_accounts` property.
    pub remaining_accounts: Vec<AccountInfo<'info>>,
    /// Will be accessible via the [`Context`]'s `bumps` property.
    pub bumps: Bumps,
    /// These data are used by the `#[instruction(...)]` macro.
    pub ix_data: Vec<u8>,
}

impl<'info> ContextWrapper<'info> {
    pub fn new(program: Pubkey) -> Self {
        Self {
            program,
            ..Default::default()
        }
    }

    /// Adds a new account - the order matters! Call this method for each
    /// account in the same order as they are defined in the endpoint's
    /// [`Accounts`] declaration struct.
    pub fn acc(mut self, account: &'info mut AccountInfoWrapper) -> Self {
        if let Some((name, bump)) = &account.pda {
            self.bumps.insert(name.clone(), *bump);
        }

        self.accounts.push(account.to_account_info());

        self
    }

    /// Set the remaining accounts attached to the [`Context`].
    pub fn remaining_accounts(
        mut self,
        accs: impl Iterator<Item = &'info mut AccountInfoWrapper>,
    ) -> Self {
        self.remaining_accounts = accs.map(|a| a.to_account_info()).collect();
        self
    }

    /// These data are used by the `#[instruction(...)]` macro.
    pub fn ix_data(mut self, data: Vec<u8>) -> Self {
        self.ix_data = data.into();
        self
    }

    pub fn accounts<T: anchor_lang::Accounts<'info>>(&mut self) -> Result<T> {
        T::try_accounts(
            &self.program,
            &mut self.accounts.as_slice(),
            &self.ix_data,
            &mut self.bumps,
        )
    }

    /// Creates the [`Context`] which can be used to call an endpoint:
    ///
    /// ```rust,ignore
    /// let mut ctx = ContextWrapper {
    ///     // TODO: set accounts
    ///     ..Default::default()
    /// };
    /// let mut accounts = ctx.accounts()?;
    ///
    /// endpoint_fn(ctx.build(&mut accounts))?;
    /// accounts.exit(&program_id)?;
    /// ```
    pub fn build<'builder, 'accs, T: anchor_lang::Accounts<'info>>(
        &'builder self,
        accounts: &'accs mut T,
    ) -> Context<'builder, 'accs, 'builder, 'info, T> {
        anchor_lang::context::Context::new(
            &self.program,
            accounts,
            &self.remaining_accounts,
            self.bumps.clone(),
        )
    }
}

/// Holds state for [`AccountInfo`].
#[derive(Default, Clone, Debug, PartialEq)]
pub struct AccountInfoWrapper {
    pub key: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
    pub lamports: u64,
    /// TODO: It would be good to have a way to specify what slice of this
    /// buffer should be passed to the [`AccountInfo`]. Then, in the
    /// [`crate::stub::ValidateCpis`] we could allow the implementor to change
    /// the size of the [`RefCell`]'s slice, just as it's done in the system
    /// program. This would enable us to mimic the Solana API more closely.
    pub data: Vec<u8>,
    pub owner: Pubkey,
    pub executable: bool,
    pub rent_epoch: u64,
    pub pda: Option<(String, u8)>,
}

impl AccountInfoWrapper {
    pub fn to_account_info<'wrapper>(
        &'wrapper mut self,
    ) -> AccountInfo<'wrapper> {
        AccountInfo {
            key: &self.key,
            is_signer: self.is_signer,
            is_writable: self.is_writable,
            lamports: Rc::new(RefCell::new(&mut self.lamports)),
            data: Rc::new(RefCell::new(&mut self.data)),
            owner: &self.owner,
            executable: self.executable,
            rent_epoch: self.rent_epoch,
        }
    }

    pub fn new() -> Self {
        Self::with_key(Pubkey::new_unique())
    }

    pub fn with_key(key: Pubkey) -> Self {
        Self {
            key,
            owner: system_program::ID,
            ..Default::default()
        }
    }

    /// # Important
    /// The program is not set as the accounts owner.
    pub fn pda(program: Pubkey, name: impl ToString, seeds: &[&[u8]]) -> Self {
        let (key, bump) = Pubkey::find_program_address(seeds, &program);
        Self {
            key,
            pda: Some((name.to_string(), bump)),
            owner: system_program::ID,
            ..Default::default()
        }
    }

    pub fn signer(mut self) -> Self {
        self.is_signer = true;
        self
    }

    pub fn mutable(mut self) -> Self {
        self.is_writable = true;
        self
    }

    /// Fill the data buffer with this many zero bytes. Useful when initing an
    /// account - it cannot be done in the stubs.
    pub fn size(mut self, space: usize) -> Self {
        self.data.resize(space, 0);
        self
    }

    pub fn program(self) -> Self {
        self.program_with_data_addr(Pubkey::new_unique())
    }

    /// [`UpgradeableLoaderState::Program`]
    pub fn program_with_data_addr(
        mut self,
        programdata_address: Pubkey,
    ) -> Self {
        self.owner = bpf_loader_upgradeable::ID;
        self.executable = true;

        self.raw(
            bincode::serialize(&UpgradeableLoaderState::Program {
                programdata_address,
            })
            .unwrap(),
        )
    }

    /// [`UpgradeableLoaderState::ProgramData`]
    pub fn program_data(self, program_authority: Pubkey) -> Self {
        self.owner(bpf_loader_upgradeable::ID).raw(
            bincode::serialize(&UpgradeableLoaderState::ProgramData {
                slot: 0,
                upgrade_authority_address: Some(program_authority),
            })
            .unwrap(),
        )
    }

    pub fn owner(mut self, owner: Pubkey) -> Self {
        self.owner = owner;
        self
    }

    /// # Note
    /// Be careful to check that the implementation of [`AccountSerialize`] is
    /// not a no-op. For some types, anchor skips serialization because it
    /// assumes that those types will never be serialized - e.g.
    /// [`UpgradeableLoaderState`].
    ///
    /// # Note
    /// Sets the lamports to 1.
    pub fn data(self, acc: impl AccountSerialize) -> Self {
        let mut data = vec![];
        acc.try_serialize(&mut data).expect("Cannot deserialize");
        self.raw(data)
    }

    /// # Note
    /// Sets the lamports to 1.
    pub fn pack<T: Pack>(self, acc: T) -> Self {
        let mut data = vec![0; T::get_packed_len()];
        acc.pack_into_slice(&mut data);
        self.raw(data)
    }

    /// # Note
    /// Sets the lamports to 1.
    pub fn raw(mut self, data: Vec<u8>) -> Self {
        self.data = data;
        self.lamports = 1;
        self
    }
}
