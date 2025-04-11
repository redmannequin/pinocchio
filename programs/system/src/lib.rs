#![no_std]
use pinocchio::{
    account_info::AccountInfo,
    cpi,
    instruction::{AccountMeta, Instruction, Signer},
};

pub mod instructions;

pinocchio_pubkey::declare_id!("11111111111111111111111111111111");

pub trait InstructionParts {
    type Accounts;
    type AccountMetas;
    type InstructionData;

    fn accounts(&self) -> Self::Accounts;
    fn account_metas(&self) -> Self::AccountMetas;
    fn instruction_data(&self) -> Self::InstructionData;

    fn instruction_data_modifer(data: &[u8]) -> &[u8] {
        data
    }
}

pub trait Invoke: Sized {
    fn invoke(self) -> pinocchio::ProgramResult {
        self.invoke_signed(&[])
    }

    fn invoke_signed(self, signers: &[Signer]) -> pinocchio::ProgramResult;
}

impl<'a, const N: usize, const M: usize, const J: usize, T> Invoke for T
where
    T: InstructionParts<
        Accounts = [&'a AccountInfo; N],
        AccountMetas = [AccountMeta<'a>; M],
        InstructionData = [u8; J],
    >,
{
    fn invoke_signed(self, signers: &[Signer]) -> pinocchio::ProgramResult {
        let data = self.instruction_data();
        let instruction = Instruction {
            program_id: &crate::ID,
            accounts: &self.account_metas(),
            data: Self::instruction_data_modifer(&data),
        };
        cpi::invoke_signed(&instruction, &self.accounts(), signers)
    }
}
