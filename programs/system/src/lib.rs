#![no_std]
use core::usize;

use pinocchio::{
    account_info::AccountInfo,
    cpi,
    instruction::{AccountMeta, Instruction, Signer},
};

pub mod instructions;

pinocchio_pubkey::declare_id!("11111111111111111111111111111111");

pub struct InvokeParts<'a, const N: usize, const M: usize, const J: usize> {
    pub accounts: [&'a AccountInfo; N],
    pub account_metas: [AccountMeta<'a>; M],
    pub instruction_data: InstructionData<J>,
}

pub enum InstructionData<const N: usize> {
    Full([u8; N]),
    Truncated(([u8; N], usize)),
}

impl<const N: usize> InstructionData<N> {
    pub fn data(&self) -> &[u8] {
        match *self {
            InstructionData::Full(ref data) => data,
            InstructionData::Truncated((ref data, end)) => &data[..end],
        }
    }
}

pub trait Invoke<'a, const N: usize, const M: usize, const J: usize>:
    Into<InvokeParts<'a, N, M, J>>
{
    fn invoke(self) -> pinocchio::ProgramResult {
        self.invoke_signed(&[])
    }

    fn invoke_signed(self, signers: &[Signer]) -> pinocchio::ProgramResult {
        invoke_invoker(self.into(), signers, |ix, acc, sigs| {
            cpi::invoke_signed(&ix, acc, sigs)
        })
    }

    unsafe fn invoke_access_unchecked(self) -> pinocchio::ProgramResult {
        self.invoke_singed_access_unchecked(&[])
    }

    unsafe fn invoke_singed_access_unchecked(self, signers: &[Signer]) -> pinocchio::ProgramResult {
        invoke_invoker(self.into(), signers, |ix, acc, sigs| unsafe {
            cpi::invoke_signed_access_unchecked(&ix, acc, sigs)
        })
    }
}

impl<'a, const N: usize, const M: usize, const J: usize, T> Invoke<'a, N, M, J> for T where
    T: Into<InvokeParts<'a, N, M, J>>
{
}

fn invoke_invoker<'a, const N: usize, const M: usize, const J: usize>(
    invoke_parts: InvokeParts<'a, N, M, J>,
    signers: &[Signer],
    invoker: impl FnOnce(Instruction, &[&AccountInfo; N], &[Signer]) -> pinocchio::ProgramResult,
) -> pinocchio::ProgramResult {
    let instruction = Instruction {
        program_id: &crate::ID,
        accounts: &invoke_parts.account_metas,
        data: invoke_parts.instruction_data.data(),
    };
    invoker(instruction, &invoke_parts.accounts, signers)
}
