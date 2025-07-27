use pinocchio::{
    account_info::AccountInfo,
    instruction::{AccountMeta, Instruction, Signer},
    program::invoke_signed,
    pubkey::Pubkey,
    ProgramResult,
};

use crate::CanInvoke;

/// Transfer lamports.
///
/// ### Accounts:
///   0. `[WRITE, SIGNER]` Funding account
///   1. `[WRITE]` Recipient account
pub struct Transfer<'a> {
    /// Funding account.
    pub from: &'a AccountInfo,

    /// Recipient account.
    pub to: &'a AccountInfo,

    /// Amount of lamports to transfer.
    pub lamports: u64,
}

impl Transfer<'_> {
    #[inline(always)]
    pub fn invoke(&self) -> ProgramResult {
        self.invoke_signed(&[])
    }

    #[inline(always)]
    pub fn invoke_signed(&self, signers: &[Signer]) -> ProgramResult {
        // account metadata
        let account_metas: [AccountMeta; 2] = [
            AccountMeta::writable_signer(self.from.key()),
            AccountMeta::writable(self.to.key()),
        ];

        // instruction data
        // -  [0..4 ]: instruction discriminator
        // -  [4..12]: lamports amount
        let mut instruction_data = [0; 12];
        instruction_data[0] = 2;
        instruction_data[4..12].copy_from_slice(&self.lamports.to_le_bytes());

        let instruction = Instruction {
            program_id: &crate::ID,
            accounts: &account_metas,
            data: &instruction_data,
        };

        invoke_signed(&instruction, &[self.from, self.to], signers)
    }
}

const ACCOUNTS_LEN: usize = 2;

impl CanInvoke<ACCOUNTS_LEN> for Transfer<'_> {
    fn invoke_via(
        &self,
        invoke: impl FnOnce(
            /* program_id: */ &Pubkey,
            /* accounts: */ &[&AccountInfo; ACCOUNTS_LEN],
            /* account_metas: */ &[AccountMeta],
            /* data: */ &[u8],
        ) -> ProgramResult,
    ) -> ProgramResult {
        let mut instruction_data = [0; 12];
        instruction_data[0] = 2;
        instruction_data[4..12].copy_from_slice(&self.lamports.to_le_bytes());

        invoke(
            &crate::ID,
            &[self.from, self.to],
            &[
                AccountMeta::writable_signer(self.from.key()),
                AccountMeta::writable(self.to.key()),
            ],
            &instruction_data,
        )
    }
}
