#![no_std]
use core::usize;

use pinocchio::{
    account_info::AccountInfo,
    cpi,
    instruction::{AccountMeta, Instruction, Signer},
    pubkey::Pubkey,
};

pub mod instructions;

pinocchio_pubkey::declare_id!("11111111111111111111111111111111");

pub struct InvokeParts<'a, const ACCOUNTS: usize, Data> {
    pub program_id: Pubkey,
    pub accounts: [&'a AccountInfo; ACCOUNTS],
    pub account_metas: [AccountMeta<'a>; ACCOUNTS],
    pub instruction_data: Data,
}

pub struct FullInstructionData<const N: usize> {
    data: [u8; N],
}

impl<const N: usize> FullInstructionData<N> {
    pub fn new(data: [u8; N]) -> Self {
        Self { data }
    }
}

pub struct TruncatedInstructionData<const N: usize> {
    data: [u8; N],
    size: usize,
}

impl<const N: usize> TruncatedInstructionData<N> {
    pub fn new(data: [u8; N], size: usize) -> Self {
        Self { data, size }
    }
}

pub trait InstructionData {
    fn as_slice(&self) -> &[u8];
    fn len(&self) -> usize;
}

impl<const N: usize> InstructionData for FullInstructionData<N> {
    fn as_slice(&self) -> &[u8] {
        &self.data
    }

    fn len(&self) -> usize {
        N
    }
}

impl<const N: usize> InstructionData for TruncatedInstructionData<N> {
    fn as_slice(&self) -> &[u8] {
        &self.data[..self.size]
    }

    fn len(&self) -> usize {
        self.size
    }
}

pub trait Invoke<'a, const ACCOUNTS: usize, Data>: Into<InvokeParts<'a, ACCOUNTS, Data>>
where
    Data: InstructionData,
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

impl<'a, const ACCOUNTS: usize, T, Data> Invoke<'a, ACCOUNTS, Data> for T
where
    T: Into<InvokeParts<'a, ACCOUNTS, Data>>,
    Data: InstructionData,
{
}

fn invoke_invoker<'a, const ACCOUNTS: usize>(
    invoke_parts: InvokeParts<'a, ACCOUNTS, impl InstructionData>,
    signers: &[Signer],
    invoker: impl FnOnce(Instruction, &[&AccountInfo; ACCOUNTS], &[Signer]) -> pinocchio::ProgramResult,
) -> pinocchio::ProgramResult {
    let instruction = Instruction {
        program_id: &invoke_parts.program_id,
        accounts: &invoke_parts.account_metas,
        data: invoke_parts.instruction_data.as_slice(),
    };
    invoker(instruction, &invoke_parts.accounts, signers)
}
