#![no_std]
use core::usize;

use pinocchio::{
    account_info::AccountInfo,
    cpi,
    instruction::{AccountMeta, Instruction, Signer},
};

pub mod instructions;

pinocchio_pubkey::declare_id!("11111111111111111111111111111111");

pub struct InvokeParts<'a, const ACCOUNTS: usize, const DATA_LEN: usize> {
    pub accounts: [&'a AccountInfo; ACCOUNTS],
    pub account_metas: [AccountMeta<'a>; ACCOUNTS],
    pub instruction_data: InstructionData<DATA_LEN>,
}

pub enum InstructionData<const N: usize> {
    Full([u8; N]),
    Truncated(([u8; N], usize)),
}

impl<const N: usize> InstructionData<N> {
    pub fn truncated(data: [u8; N], end: usize) -> Self {
        InstructionData::Truncated((data, end))
    }

    pub fn as_slice(&self) -> &[u8] {
        match *self {
            InstructionData::Full(ref data) => data,
            InstructionData::Truncated((ref data, end)) => &data[..end],
        }
    }

    pub fn len(&self) -> usize {
        match *self {
            InstructionData::Full(_) => N,
            InstructionData::Truncated((_, len)) => len,
        }
    }
}

pub trait Invoke<'a, const ACCOUNTS: usize, const DATA_LEN: usize>:
    Into<InvokeParts<'a, ACCOUNTS, DATA_LEN>>
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

impl<'a, const ACCOUNTS: usize, const DATA_LEN: usize, T> Invoke<'a, ACCOUNTS, DATA_LEN> for T where
    T: Into<InvokeParts<'a, ACCOUNTS, DATA_LEN>>
{
}

fn invoke_invoker<'a, const ACCOUNTS: usize, const DATA_LEN: usize>(
    invoke_parts: InvokeParts<'a, ACCOUNTS, DATA_LEN>,
    signers: &[Signer],
    invoker: impl FnOnce(Instruction, &[&AccountInfo; ACCOUNTS], &[Signer]) -> pinocchio::ProgramResult,
) -> pinocchio::ProgramResult {
    let instruction = Instruction {
        program_id: &crate::ID,
        accounts: &invoke_parts.account_metas,
        data: invoke_parts.instruction_data.as_slice(),
    };
    invoker(instruction, &invoke_parts.accounts, signers)
}
