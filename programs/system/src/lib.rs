#![no_std]

use pinocchio::{
    account_info::AccountInfo,
    instruction::{AccountMeta, Instruction, Signer},
    pubkey::Pubkey,
    ProgramResult,
};

pub mod instructions;

pinocchio_pubkey::declare_id!("11111111111111111111111111111111");

pub trait CanInvoke<const ACCOUNTS_LEN: usize>: Sized {
    fn invoke_via(
        self,
        invoke: impl FnOnce(
            /* program_id: */ &Pubkey,
            /* accounts: */ &[&AccountInfo; ACCOUNTS_LEN],
            /* account_metas: */ &[AccountMeta],
            /* data: */ &[u8],
        ) -> ProgramResult,
    ) -> ProgramResult;

    fn invoke(self) -> ProgramResult {
        self.invoke_via(|program_id, accounts, account_metas, data| {
            let instruction = Instruction {
                program_id: program_id,
                accounts: &account_metas,
                data: data,
            };
            pinocchio::cpi::invoke(&instruction, accounts)
        })
    }

    fn invoke_signed(self, signers: &[Signer]) -> ProgramResult {
        self.invoke_via(|program_id, accounts, account_metas, data| {
            let instruction = Instruction {
                program_id: program_id,
                accounts: &account_metas,
                data: data,
            };
            pinocchio::cpi::invoke_signed(&instruction, accounts, signers)
        })
    }
}
