use clap::{Parser, Subcommand};
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Parser)]
#[command(name = "mps_pkg")]
#[command(about = "Makes Python Slow (MPS) Package Manager", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new MPS project
    Init,
    /// Add a dependency from a GitHub repository
    Add {
        /// GitHub repository URL (e.g., https://github.com/user/repo)
        url: String,
    },
    /// Install all dependencies specified in mps.toml
    Install,
}

#[derive(Serialize, Deserialize, Debug)]
struct ProjectConfig {
    package: PackageInfo,
    #[serde(default)]
    dependencies: std::collections::BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct PackageInfo {
    name: String,
    version: String,
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Init => {
            let config_path = Path::new("mps.toml");
            if config_path.exists() {
                println!("Error: mps.toml already exists in this directory.");
                std::process::exit(1);
            }
            
            let current_dir_name = std::env::current_dir()
                .ok()
                .and_then(|p| p.file_name().map(|s| s.to_string_lossy().to_string()))
                .unwrap_or_else(|| "my_project".to_string());
                
            let template = format!(
                "[package]\nname = \"{}\"\nversion = \"0.1.0\"\n\n[dependencies]\n",
                current_dir_name
            );
            
            if let Err(e) = fs::write(config_path, template) {
                eprintln!("Error: Failed to write mps.toml: {}", e);
                std::process::exit(1);
            }
            println!("Initialized new MPS project in current directory.");
        }
        Commands::Add { url } => {
            let config_path = Path::new("mps.toml");
            if !config_path.exists() {
                println!("Error: No mps.toml found. Run 'mps_pkg init' first.");
                std::process::exit(1);
            }
            
            let content = fs::read_to_string(config_path).unwrap();
            let mut config: ProjectConfig = toml::from_str(&content).unwrap_or_else(|e| {
                eprintln!("Error: Failed to parse mps.toml: {}", e);
                std::process::exit(1);
            });
            
            // Extract repo name from URL
            let repo_name = url
                .trim_end_matches('/')
                .split('/')
                .last()
                .unwrap_or("dependency")
                .trim_end_matches(".git")
                .to_string();
                
            config.dependencies.insert(repo_name.clone(), url.clone());
            
            let updated = toml::to_string(&config).unwrap();
            fs::write(config_path, updated).unwrap();
            
            println!("Added dependency '{}' ({}) to mps.toml.", repo_name, url);
        }
        Commands::Install => {
            let config_path = Path::new("mps.toml");
            if !config_path.exists() {
                println!("Error: No mps.toml found.");
                std::process::exit(1);
            }
            
            let content = fs::read_to_string(config_path).unwrap();
            let config: ProjectConfig = toml::from_str(&content).unwrap_or_else(|e| {
                eprintln!("Error: Failed to parse mps.toml: {}", e);
                std::process::exit(1);
            });
            
            // Setup cache directory
            let home_dir = std::env::var("USERPROFILE")
                .or_else(|_| std::env::var("HOME"))
                .unwrap_or_else(|_| ".".to_string());
            let cache_dir = PathBuf::from(home_dir).join(".mps").join("packages");
            fs::create_dir_all(&cache_dir).unwrap();
            
            println!("Resolving and installing dependencies...");
            for (name, url) in &config.dependencies {
                let pkg_dir = cache_dir.join(name);
                if pkg_dir.exists() {
                    println!("Package '{}' already cached at {}", name, pkg_dir.display());
                    continue;
                }
                
                println!("Cloning '{}' from {} ...", name, url);
                let status = Command::new("git")
                    .args(["clone", "--depth", "1", url, pkg_dir.to_str().unwrap()])
                    .status();
                    
                match status {
                    Ok(s) if s.success() => {
                        println!("Successfully installed '{}'.", name);
                    }
                    _ => {
                        eprintln!("Error: Failed to clone package '{}' from '{}'.", name, url);
                    }
                }
            }
            
            // Create lock file
            let lock_content = format!(
                "# Auto-generated lockfile for MPS\n# Do not edit manually.\n\n[resolved]\n"
            );
            fs::write("mps.lock", lock_content).ok();
            println!("Dependencies synchronized successfully. Created mps.lock.");
        }
    }
}
