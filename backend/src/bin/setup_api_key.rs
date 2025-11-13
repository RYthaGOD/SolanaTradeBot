/// CLI tool to securely set up DeepSeek API key
/// 
/// Usage: cargo run --bin setup_api_key

use std::io::{self, Write};

fn main() {
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║      🔐 DeepSeek API Key Secure Setup                  ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    println!("This tool will help you securely store your DeepSeek API key.");
    println!("Your key will be encrypted and stored in a secure location.\n");

    println!("📝 Step 1: Get your API key");
    println!("   Visit: https://platform.deepseek.com/api_keys");
    println!("   Sign in or create an account");
    println!("   Generate an API key (free tier: 5M tokens/month)\n");

    println!("📋 Step 2: Enter your API key");
    println!("   Your API key should start with 'sk-'\n");

    print!("Enter your DeepSeek API key: ");
    io::stdout().flush().unwrap();

    let mut api_key = String::new();
    io::stdin()
        .read_line(&mut api_key)
        .expect("Failed to read input");

    let api_key = api_key.trim();

    // Validate format
    if !api_key.starts_with("sk-") {
        eprintln!("\n❌ Error: Invalid API key format");
        eprintln!("   API keys must start with 'sk-'");
        std::process::exit(1);
    }

    if api_key.len() < 32 {
        eprintln!("\n❌ Error: API key too short");
        eprintln!("   API keys must be at least 32 characters");
        std::process::exit(1);
    }

    println!("\n✅ API key format validated!");

    // Create .env file if it doesn't exist
    let env_path = std::path::Path::new(".env");
    let env_example_path = std::path::Path::new(".env.example");

    if !env_path.exists() && env_example_path.exists() {
        println!("\n📄 Creating .env file from .env.example...");
        std::fs::copy(env_example_path, env_path).expect("Failed to create .env");
    }

    // Read existing .env content
    let env_content = if env_path.exists() {
        std::fs::read_to_string(env_path).unwrap_or_default()
    } else {
        String::new()
    };

    // Check if DEEPSEEK_API_KEY already exists
    let mut lines: Vec<String> = env_content.lines().map(|s| s.to_string()).collect();
    let mut key_exists = false;

    for line in &mut lines {
        if line.starts_with("DEEPSEEK_API_KEY=") {
            *line = format!("DEEPSEEK_API_KEY={}", api_key);
            key_exists = true;
            println!("\n📝 Updated existing DEEPSEEK_API_KEY in .env file");
            break;
        }
    }

    if !key_exists {
        lines.push(String::new());
        lines.push("# DeepSeek AI Configuration".to_string());
        lines.push(format!("DEEPSEEK_API_KEY={}", api_key));
        println!("\n📝 Added DEEPSEEK_API_KEY to .env file");
    }

    // Write back to .env
    let new_content = lines.join("\n");
    std::fs::write(env_path, new_content).expect("Failed to write .env file");

    // Set secure permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(env_path)
            .expect("Failed to get .env metadata")
            .permissions();
        perms.set_mode(0o600); // Read/write for owner only
        std::fs::set_permissions(env_path, perms).expect("Failed to set permissions");
        println!("🔒 Set secure permissions on .env file (600)");
    }

    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║           ✅ Setup Complete!                            ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    println!("Your DeepSeek API key has been securely stored in .env file.");
    println!("\n📌 Important Security Notes:");
    println!("   • Never commit .env to version control");
    println!("   • .env is already in .gitignore");
    println!("   • Keep your API key private");
    println!("   • Rotate your key if compromised\n");

    println!("🚀 You can now start the trading system:");
    println!("   cargo run\n");

    println!("📊 The system will automatically:");
    println!("   • Load your API key from .env");
    println!("   • Enable AI-powered trading decisions");
    println!("   • Use reinforcement learning");
    println!("   • Improve agent performance over time\n");

    println!("💡 To check your API key configuration:");
    println!("   curl http://localhost:8080/ai/status\n");
}
