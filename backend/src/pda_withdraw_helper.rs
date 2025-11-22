//! Helper for PDA withdrawals using a Solana program
//! Since invoke_signed is program-side only, we need a program to handle withdrawals
//! 
//! This module provides utilities for calling a withdrawal program or
//! constructing transactions that can work with a Solana program

use solana_sdk::{
    pubkey::Pubkey,
    instruction::{Instruction, AccountMeta},
    system_program,
};
use std::str::FromStr;

/// Helper to create a withdrawal instruction for a Solana program
/// This instruction would be processed by a Solana program that uses invoke_signed
pub struct PDAWithdrawHelper;

impl PDAWithdrawHelper {
    /// Create instruction data for a withdrawal program
    /// The program would process this and use invoke_signed to transfer from PDA
    pub fn create_withdraw_instruction(
        program_id: &Pubkey,
        pda: &Pubkey,
        authority: &Pubkey,
        destination: &Pubkey,
        amount_lamports: u64,
        bump: u8,
    ) -> Instruction {
        // Instruction data format for the withdrawal program
        // Format: [instruction_discriminator: u8, amount: u64, bump: u8]
        let mut instruction_data = Vec::with_capacity(10);
        instruction_data.push(0); // Instruction discriminator (0 = withdraw)
        instruction_data.extend_from_slice(&amount_lamports.to_le_bytes());
        instruction_data.push(bump);
        
        Instruction {
            program_id: *program_id,
            accounts: vec![
                AccountMeta::new(*pda, false),           // PDA account (writable, signer via seeds)
                AccountMeta::new(*destination, false),   // Destination wallet (writable)
                AccountMeta::new_readonly(*authority, true), // Authority wallet (signer)
                AccountMeta::new_readonly(system_program::id(), false), // System Program
            ],
            data: instruction_data,
        }
    }
    
    /// Get the PDA seeds for signing
    pub fn get_pda_seeds(authority: &Pubkey, bump: u8) -> Vec<Vec<u8>> {
        vec![
            b"agent-treasury".to_vec(),
            authority.as_ref().to_vec(),
            vec![bump],
        ]
    }
    
    /// Get signer seeds array format for invoke_signed
    /// Format expected by invoke_signed: &[&[&[u8]]]
    pub fn get_signer_seeds(authority: &Pubkey, bump: u8) -> Vec<Vec<u8>> {
        Self::get_pda_seeds(authority, bump)
    }
}

/// Example Solana Program Code (to be deployed separately):
/// 
/// ```rust
/// use solana_program::{
///     account_info::{next_account_info, AccountInfo},
///     entrypoint,
///     entrypoint::ProgramResult,
///     program::invoke_signed,
///     pubkey::Pubkey,
///     system_instruction,
/// };
/// 
/// entrypoint!(process_instruction);
/// 
/// fn process_instruction(
///     program_id: &Pubkey,
///     accounts: &[AccountInfo],
///     instruction_data: &[u8],
/// ) -> ProgramResult {
///     let account_info_iter = &mut accounts.iter();
///     let pda_account = next_account_info(account_info_iter)?;
///     let destination_account = next_account_info(account_info_iter)?;
///     let authority_account = next_account_info(account_info_iter)?;
///     let system_program = next_account_info(account_info_iter)?;
///     
///     // Verify authority is a signer
///     if !authority_account.is_signer {
///         return Err(ProgramError::MissingRequiredSignature);
///     }
///     
///     // Parse instruction data
///     let amount_lamports = u64::from_le_bytes(
///         instruction_data[1..9].try_into().unwrap()
///     );
///     let bump = instruction_data[9];
///     
///     // Verify PDA derivation
///     let seeds = &[
///         b"agent-treasury",
///         authority_account.key.as_ref(),
///         &[bump],
///     ];
///     let (expected_pda, _) = Pubkey::find_program_address(seeds, program_id);
///     if pda_account.key != &expected_pda {
///         return Err(ProgramError::InvalidSeeds);
///     }
///     
///     // Create transfer instruction FROM PDA TO destination
///     let transfer_instruction = system_instruction::transfer(
///         pda_account.key,
///         destination_account.key,
///         amount_lamports,
///     );
///     
///     // Invoke with PDA signing using seeds
///     invoke_signed(
///         &transfer_instruction,
///         &[
///             pda_account.clone(),
///             destination_account.clone(),
///             system_program.clone(),
///         ],
///         &[&[
///             b"agent-treasury",
///             authority_account.key.as_ref(),
///             &[bump],
///         ]],
///     )?;
///     
///     Ok(())
/// }
/// ```








