use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Treasury PDA (Program Derived Address) manager
/// PDAs are deterministic addresses derived from a program ID and seeds
pub struct TreasuryPDA {
    /// The derived PDA address
    pub address: Pubkey,
    /// The bump seed used to derive the PDA
    pub bump: u8,
    /// The authority that controls this PDA
    pub authority: Pubkey,
}

impl TreasuryPDA {
    /// Derive a PDA for the agent trading treasury
    /// 
    /// # Arguments
    /// * `program_id` - The program ID that owns this PDA
    /// * `authority` - The authority (wallet) that controls this treasury
    /// * `seed_prefix` - A prefix for the seed (e.g., "treasury", "agent-vault")
    pub fn derive(program_id: &Pubkey, authority: &Pubkey, seed_prefix: &str) -> Result<Self, String> {
        // Create seeds for PDA derivation
        let seeds = &[
            seed_prefix.as_bytes(),
            authority.as_ref(),
        ];

        // Find PDA with bump seed
        let (address, bump) = Pubkey::find_program_address(seeds, program_id);

        log::info!("🏦 Derived Treasury PDA: {}", address);
        log::info!("   Authority: {}", authority);
        log::info!("   Bump: {}", bump);

        Ok(Self {
            address,
            bump,
            authority: *authority,
        })
    }

    /// Derive the default agent treasury PDA for mainnet/devnet trading
    /// Uses the System Program as the default program ID
    pub fn derive_default(authority: &Pubkey) -> Result<Self, String> {
        // Use System Program ID as default
        let system_program = solana_sdk::system_program::id();
        Self::derive(&system_program, authority, "agent-treasury")
    }

    /// Derive a PDA for a specific agent by name
    pub fn derive_for_agent(authority: &Pubkey, agent_name: &str) -> Result<Self, String> {
        let system_program = solana_sdk::system_program::id();
        let seed = format!("agent-{}", agent_name);
        Self::derive(&system_program, authority, &seed)
    }

    /// Verify that this PDA was correctly derived
    pub fn verify(&self, program_id: &Pubkey, seed_prefix: &str) -> bool {
        let seeds = &[
            seed_prefix.as_bytes(),
            self.authority.as_ref(),
            &[self.bump],
        ];

        // Create the address from seeds and verify it matches
        match Pubkey::create_program_address(seeds, program_id) {
            Ok(derived) => derived == self.address,
            Err(_) => false,
        }
    }

    /// Get the PDA address as a string
    pub fn address_string(&self) -> String {
        self.address.to_string()
    }
}

/// Helper function to parse a program ID from string
pub fn parse_program_id(program_id_str: &str) -> Result<Pubkey, String> {
    Pubkey::from_str(program_id_str)
        .map_err(|e| format!("Invalid program ID: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::signature::{Keypair, Signer};

    #[test]
    fn test_derive_default_pda() {
        let keypair = Keypair::new();
        let authority = keypair.pubkey();
        
        let pda = TreasuryPDA::derive_default(&authority).unwrap();
        
        assert_eq!(pda.authority, authority);
        assert!(pda.bump <= 255);
        assert_ne!(pda.address, authority); // PDA should be different from authority
    }

    #[test]
    fn test_derive_pda_deterministic() {
        let keypair = Keypair::new();
        let authority = keypair.pubkey();
        let program_id = solana_sdk::system_program::id();
        
        let pda1 = TreasuryPDA::derive(&program_id, &authority, "test-treasury").unwrap();
        let pda2 = TreasuryPDA::derive(&program_id, &authority, "test-treasury").unwrap();
        
        // Should derive the same PDA
        assert_eq!(pda1.address, pda2.address);
        assert_eq!(pda1.bump, pda2.bump);
    }

    #[test]
    fn test_derive_for_agent() {
        let keypair = Keypair::new();
        let authority = keypair.pubkey();
        
        let pda1 = TreasuryPDA::derive_for_agent(&authority, "oracle-agent").unwrap();
        let pda2 = TreasuryPDA::derive_for_agent(&authority, "dex-agent").unwrap();
        
        // Different agent names should produce different PDAs
        assert_ne!(pda1.address, pda2.address);
    }

    #[test]
    fn test_verify_pda() {
        let keypair = Keypair::new();
        let authority = keypair.pubkey();
        let program_id = solana_sdk::system_program::id();
        let seed = "test-treasury";
        
        let pda = TreasuryPDA::derive(&program_id, &authority, seed).unwrap();
        
        // Verification should succeed
        assert!(pda.verify(&program_id, seed));
    }

    #[test]
    fn test_address_string() {
        let keypair = Keypair::new();
        let authority = keypair.pubkey();
        
        let pda = TreasuryPDA::derive_default(&authority).unwrap();
        let address_str = pda.address_string();
        
        assert!(!address_str.is_empty());
        // Should be able to parse it back
        let parsed = Pubkey::from_str(&address_str).unwrap();
        assert_eq!(parsed, pda.address);
    }

    #[test]
    fn test_parse_program_id() {
        let valid_id = "11111111111111111111111111111111";
        assert!(parse_program_id(valid_id).is_ok());
        
        let invalid_id = "invalid-program-id";
        assert!(parse_program_id(invalid_id).is_err());
    }
}
