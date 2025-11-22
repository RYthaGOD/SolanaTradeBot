# PDA Withdrawal Setup Guide

## Overview

The PDA withdrawal feature requires a Solana program that uses `invoke_signed` to transfer funds from the PDA treasury back to your wallet. According to the [Solana documentation](https://docs.rs/solana-cpi/latest/solana_cpi/fn.invoke_signed.html), `invoke_signed` is program-side only and must be called from within a Solana program context.

## Why a Program is Needed

Program Derived Addresses (PDAs) don't have private keys, so they can't sign transactions directly. They must sign using seeds via `invoke_signed`, which can only be called from within a Solana program.

## Setup Instructions

### 1. Create and Deploy a Withdrawal Program

You need to create a Solana program with a withdraw function. Example code structure:

```rust
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    program::invoke_signed,
    pubkey::Pubkey,
    system_instruction,
};

entrypoint!(process_instruction);

fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let pda_account = next_account_info(account_info_iter)?;
    let destination_account = next_account_info(account_info_iter)?;
    let authority_account = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    
    // Verify authority is a signer
    if !authority_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    // Parse instruction data: [discriminator: u8, amount: u64, bump: u8]
    if instruction_data.len() < 10 {
        return Err(ProgramError::InvalidInstructionData);
    }
    
    let amount_lamports = u64::from_le_bytes(
        instruction_data[1..9].try_into().unwrap()
    );
    let bump = instruction_data[9];
    
    // Verify PDA derivation
    let seeds = &[
        b"agent-treasury",
        authority_account.key.as_ref(),
        &[bump],
    ];
    let (expected_pda, _) = Pubkey::find_program_address(seeds, program_id);
    if pda_account.key != &expected_pda {
        return Err(ProgramError::InvalidSeeds);
    }
    
    // Create transfer instruction FROM PDA TO destination
    let transfer_instruction = system_instruction::transfer(
        pda_account.key,
        destination_account.key,
        amount_lamports,
    );
    
    // Invoke with PDA signing using seeds
    invoke_signed(
        &transfer_instruction,
        &[
            pda_account.clone(),
            destination_account.clone(),
            system_program.clone(),
        ],
        &[&[
            b"agent-treasury",
            authority_account.key.as_ref(),
            &[bump],
        ]],
    )?;
    
    Ok(())
}
```

### 2. Deploy the Program

Deploy your program to Solana (devnet or mainnet) and note the program ID.

### 3. Set Environment Variable

Add the program ID to your `.env` file:

```bash
WITHDRAW_PROGRAM_ID=YourProgramIdHere
```

### 4. Use the Withdrawal API

Once configured, you can use the withdrawal endpoint:

```bash
curl -X POST http://localhost:8080/pda/withdraw \
  -H "Content-Type: application/json" \
  -d '{"amount_sol": 1.0}'
```

## How It Works

1. Client creates a transaction with an instruction to call your withdrawal program
2. Instruction includes:
   - PDA account (signer via seeds)
   - Destination wallet (writable)
   - Authority wallet (signer - must match PDA authority)
   - System Program
   - Instruction data: `[0, amount_le_bytes, bump]`
3. Your program receives the instruction
4. Program verifies the authority signature
5. Program uses `invoke_signed` with PDA seeds to transfer funds
6. Transaction completes successfully

## Current Status

✅ **Deposit to PDA** - Works immediately, no program needed  
✅ **Check PDA Balance** - Works immediately  
✅ **Get PDA Info** - Works immediately  
⚠️ **Withdraw from PDA** - Requires withdrawal program (see above)

## Example Helper Code

See `backend/src/pda_withdraw_helper.rs` for example code and detailed implementation notes.

## References

- [Solana invoke_signed Documentation](https://docs.rs/solana-cpi/latest/solana_cpi/fn.invoke_signed.html)
- [Solana Program Derived Addresses](https://docs.solana.com/developing/programming-model/calling-between-programs#program-derived-addresses)








