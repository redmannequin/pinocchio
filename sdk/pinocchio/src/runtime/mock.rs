use std::{
    collections::HashMap,
    eprintln, format,
    string::String,
    sync::{Arc, LazyLock, Mutex},
    vec::Vec,
};

use crate::{
    account_info::{Account, AccountInfo, MAX_PERMITTED_DATA_INCREASE},
    instruction::{Instruction, Signer},
    log::sol_log,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvars::rent::Rent,
    ProgramResult,
};

use super::Runtime;

pub static MOCK_RUNTIME: LazyLock<Mutex<MockRuntime>> =
    LazyLock::new(|| Mutex::new(MockRuntime::init()));

pub trait MockProgram: Fn(&Pubkey, &[AccountInfo], &[u8]) -> ProgramResult + Sized {
    fn wrap(
        self,
        before: impl Fn(&Pubkey, &[AccountInfo], &[u8]),
        after: impl Fn(&ProgramResult),
    ) -> impl Fn(&Pubkey, &[AccountInfo], &[u8]) -> ProgramResult {
        move |key, accounts, payload| {
            before(key, accounts, payload);
            let res = self(key, accounts, payload);
            after(&res);
            res
        }
    }
}

impl<T> MockProgram for T where T: Fn(&Pubkey, &[AccountInfo], &[u8]) -> ProgramResult {}

pub type ArcMockProgram =
    Arc<dyn Fn(&Pubkey, &[AccountInfo], &[u8]) -> ProgramResult + Sync + Send + 'static>;

pub type MockProgramAccount = MockAccount<ArcMockProgram>;
pub type MockDataAccount = MockAccount<Vec<u8>>;

pub struct MockAccount<T> {
    is_signer: bool,
    is_writable: bool,
    key: Pubkey,
    owener: Pubkey,
    lamports: u64,
    data: T,
}

impl MockProgramAccount {
    pub fn new_program(
        is_signer: bool,
        is_writable: bool,
        key: Pubkey,
        owener: Pubkey,
        lamports: u64,
        program: impl MockProgram + Sync + Send + 'static,
    ) -> Self {
        MockAccount {
            is_signer,
            is_writable,
            key,
            owener,
            lamports,
            data: Arc::new(program),
        }
    }
}
impl MockDataAccount {
    pub fn new_data_account(
        is_signer: bool,
        is_writable: bool,
        key: Pubkey,
        owener: Pubkey,
        lamports: u64,
        data: Vec<u8>,
    ) -> Self {
        MockAccount {
            is_signer,
            is_writable,
            key,
            owener,
            lamports,
            data,
        }
    }

    fn to_bytes(self) -> Vec<u8> {
        let mut raw_data = Vec::from([0, self.is_signer as u8, self.is_writable as u8, 0]);
        raw_data.extend((self.data.len() as u32).to_ne_bytes());
        raw_data.extend(self.key);
        raw_data.extend(self.owener);
        raw_data.extend(self.lamports.to_ne_bytes());
        raw_data.extend((self.data.len() as u64).to_ne_bytes());
        raw_data.extend(self.data);
        raw_data.reserve_exact(MAX_PERMITTED_DATA_INCREASE);
        raw_data
    }
}

struct AccountMap {
    account_type: AccountType,
    id: usize,
    name: &'static str,
}

enum AccountType {
    Program,
    Data,
}

pub struct MockRuntime {
    accounts: HashMap<Pubkey, AccountMap>,
    program_accounts: Vec<MockProgramAccount>,
    data_accounts: Vec<Vec<u8>>,
    logs: Vec<String>,
    compute_units: u64,
    rent: Rent,
}

impl MockRuntime {
    pub fn init() -> Self {
        MockRuntime {
            accounts: HashMap::new(),
            program_accounts: Vec::new(),
            data_accounts: Vec::new(),
            logs: Vec::new(),
            compute_units: 0,
            rent: Rent {
                lamports_per_byte_year: 0,
                exemption_threshold: 0.0,
                burn_percent: 0,
            },
        }
    }

    pub fn register_program_account(&mut self, name: &'static str, program: MockProgramAccount) {
        let id = self.program_accounts.len();
        self.accounts.insert(
            program.key,
            AccountMap {
                account_type: AccountType::Program,
                id,
                name,
            },
        );
        self.program_accounts.push(program);
    }

    pub fn register_data_account(&mut self, name: &'static str, account: MockDataAccount) {
        let id = self.data_accounts.len();
        self.accounts.insert(
            account.key,
            AccountMap {
                account_type: AccountType::Data,
                id,
                name,
            },
        );
        self.data_accounts.push(account.to_bytes());
    }

    pub fn get_data_account(&mut self, key: &crate::pubkey::Pubkey) -> Option<AccountInfo> {
        self.accounts
            .get_mut(key)
            .and_then(|acc| match acc.account_type {
                AccountType::Program => None,
                AccountType::Data => Some(acc.id),
            })
            .map(|id| {
                let raw_account_ptr = self.data_accounts[id].as_mut_ptr();
                AccountInfo {
                    raw: raw_account_ptr as *mut Account,
                }
            })
    }
}

pub fn invoke<const ACCOUNTS: usize>(
    instruction: &Instruction,
    account_infos: &[&AccountInfo; ACCOUNTS],
) {
    let (name, program) = {
        let rt_lock = MOCK_RUNTIME.lock();
        let rt = rt_lock.unwrap();

        let account = rt
            .accounts
            .get(instruction.program_id)
            .expect("program not found");

        if let AccountType::Data = account.account_type {
            panic!("invalid program id")
        };

        (account.name, rt.program_accounts[account.id].data.clone())
    };

    let accounts: Vec<_> = account_infos.iter().map(|acc| (*acc).clone()).collect();

    program.wrap(
        |_, _, _| {
            sol_log(&format!("Enter {}", name));
        },
        |_| {
            sol_log(&format!("Exit {}", name));
        },
    )(
        instruction.program_id,
        accounts.as_slice(),
        instruction.data,
    )
    .map_err(|err| {
        eprintln!("Err: {:?}", err);
        eprintln!("---- LOG ----");
        let rt = MOCK_RUNTIME.lock().unwrap();
        for log in rt.logs.iter() {
            eprintln!("{}", log);
        }
    })
    .expect("program failed to execute");
}

impl Runtime for MockRuntime {
    ////////////////////////////////////////////////////////////////////////////
    // LOG SYS CALLS
    ////////////////////////////////////////////////////////////////////////////

    fn sol_log(message: &str) {
        MOCK_RUNTIME.lock().unwrap().logs.push(message.into());
    }

    fn sol_log_64(arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64) {
        let rt_lock = MOCK_RUNTIME.lock();
        let mut rt = rt_lock.unwrap();
        rt.logs.push(format!(
            "Program log: {:x} {:x} {:x} {:x} {:x}",
            arg1, arg2, arg3, arg4, arg5
        ));
    }

    fn sol_log_data(data: &[&[u8]]) {
        MOCK_RUNTIME
            .lock()
            .unwrap()
            .logs
            .push(format!("data: {:?}", data));
    }

    fn sol_log_compute_units() {
        let rt_lock = MOCK_RUNTIME.lock();
        let mut rt = rt_lock.unwrap();
        let cu = rt.compute_units;
        rt.logs.push(format!("cu: {}", cu));
    }

    ////////////////////////////////////////////////////////////////////////////
    // MEM SYS CALLS
    ////////////////////////////////////////////////////////////////////////////

    unsafe fn sol_memcpy(_dst: &mut [u8], _src: &[u8], _n: usize) {
        unimplemented!()
    }

    unsafe fn sol_memmove(_dst: *mut u8, _src: *mut u8, _n: usize) {
        unimplemented!()
    }

    unsafe fn sol_memcmp(_s1: &[u8], _s2: &[u8], _n: usize) -> i32 {
        unimplemented!()
    }

    unsafe fn sol_memset(_s: &mut [u8], _c: u8, _n: usize) {
        unimplemented!()
    }

    ////////////////////////////////////////////////////////////////////////////
    // CPI CALLS
    ////////////////////////////////////////////////////////////////////////////

    fn invoke_signed<const ACCOUNTS: usize>(
        instruction: &Instruction,
        account_infos: &[&AccountInfo; ACCOUNTS],
        _signers_seeds: &[Signer],
    ) -> ProgramResult {
        if instruction.accounts.len() < ACCOUNTS {
            return Err(ProgramError::NotEnoughAccountKeys);
        }

        for index in 0..ACCOUNTS {
            let account_info = account_infos[index];
            let account_meta = &instruction.accounts[index];

            if account_info.key() != account_meta.pubkey {
                return Err(ProgramError::InvalidArgument);
            }

            if account_meta.is_writable {
                account_info.check_borrow_mut_data()?;
                account_info.check_borrow_mut_lamports()?;
            } else {
                account_info.check_borrow_data()?;
                account_info.check_borrow_lamports()?;
            }
        }

        invoke(instruction, account_infos);

        Ok(())
    }

    unsafe fn invoke_signed_access_unchecked<const ACCOUNTS: usize>(
        instruction: &Instruction,
        account_infos: &[&AccountInfo; ACCOUNTS],
        _signers_seeds: &[Signer],
    ) -> ProgramResult {
        if instruction.accounts.len() < ACCOUNTS {
            return Err(ProgramError::NotEnoughAccountKeys);
        }

        for index in 0..ACCOUNTS {
            let account_info = account_infos[index];
            let account_meta = &instruction.accounts[index];
            if account_info.key() != account_meta.pubkey {
                return Err(ProgramError::InvalidArgument);
            }
        }

        invoke(instruction, account_infos);

        Ok(())
    }

    ////////////////////////////////////////////////////////////////////////////
    // Addressing CALLS
    ////////////////////////////////////////////////////////////////////////////

    fn sol_create_program_address(
        seeds: &[&[u8]],
        program_id: &Pubkey,
    ) -> Result<Pubkey, ProgramError> {
        solana_pubkey::Pubkey::create_program_address(
            seeds,
            &solana_pubkey::Pubkey::new_from_array(*program_id),
        )
        .map(|key| key.to_bytes())
        .map_err(|err| match err {
            solana_pubkey::PubkeyError::MaxSeedLengthExceeded => {
                ProgramError::MaxSeedLengthExceeded
            }
            solana_pubkey::PubkeyError::InvalidSeeds => ProgramError::InvalidSeeds,
            solana_pubkey::PubkeyError::IllegalOwner => ProgramError::IllegalOwner,
        })
    }

    fn try_find_program_address(seeds: &[&[u8]], program_id: &Pubkey) -> Option<(Pubkey, u8)> {
        solana_pubkey::Pubkey::try_find_program_address(
            seeds,
            &solana_pubkey::Pubkey::new_from_array(*program_id),
        )
        .map(|(key, bump)| (key.to_bytes(), bump))
    }

    ////////////////////////////////////////////////////////////////////////////
    // SYSVAR CALLS
    ////////////////////////////////////////////////////////////////////////////

    fn sol_get_rent_sysvar() -> Result<Rent, ProgramError> {
        Ok(MOCK_RUNTIME.lock().unwrap().rent.clone())
    }
}
