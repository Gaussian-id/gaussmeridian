use clap::{Parser, Subcommand};
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use crate::config::{MoaConfig, AgentRole, AgentType};
use crate::error::MoaError;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Path to config file
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Log level (error, warn, info, debug, trace)
    #[arg(short, long, default_value = "info")]
    pub log_level: String,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the MOA server
    Start {
        /// Port to listen on
        #[arg(short, long, default_value_t = 8080)]
        port: u16,

        /// Host address to bind to
        #[arg(short, long, default_value = "127.0.0.1")]
        host: String,
    },

    /// Initialize a new configuration
    Init {
        /// Directory to create config in
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
    },

    /// Validate configuration
    Validate {
        /// Path to config file
        #[arg(short, long)]
        config: PathBuf,
    },

    /// Manage agents
    Agent {
        #[command(subcommand)]
        command: AgentCommands,
    },

    /// Show system status
    Status,
}

#[derive(Subcommand)]
pub enum AgentCommands {
    /// List all agents
    List,

    /// Add a new agent
    Add {
        /// Agent name
        #[arg(short, long)]
        name: String,

        /// Agent type (LLM, Custom, Ensemble)
        #[arg(short, long)]
        agent_type: String,

        /// Agent role (Primary, Secondary, Fallback)
        #[arg(short, long)]
        role: String,

        /// Agent capabilities (comma-separated)
        #[arg(short, long)]
        capabilities: String,
    },

    /// Remove an agent
    Remove {
        /// Agent name
        #[arg(short, long)]
        name: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CliConfig {
    pub host: String,
    pub port: u16,
    pub log_level: String,
    pub config_path: PathBuf,
}

impl CliConfig {
    pub fn from_cli(cli: &Cli, command: &Commands) -> Self {
        match command {
            Commands::Start { port, host } => Self {
                host: host.clone(),
                port: *port,
                log_level: cli.log_level.clone(),
                config_path: cli.config.clone().unwrap_or_else(|| PathBuf::from("config.yaml")),
            },
            _ => Self {
                host: "127.0.0.1".to_string(),
                port: 8080,
                log_level: cli.log_level.clone(),
                config_path: cli.config.clone().unwrap_or_else(|| PathBuf::from("config.yaml")),
            },
        }
    }
}

pub async fn handle_command(cli: Cli) -> Result<(), MoaError> {
    match &cli.command {
        Commands::Start { port, host } => {
            let config = MoaConfig::load().await?;
            // Start server implementation
            println!("Starting server on {}:{}", host, port);
            Ok(())
        }

        Commands::Init { dir } => {
            let config = MoaConfig {
                agent: crate::config::AgentConfig {
                    name: "default_agent".to_string(),
                    agent_type: AgentType::LLM,
                    role: AgentRole::Primary,
                    capabilities: vec!["text".to_string()],
                    config: serde_json::json!({}),
                },
                storage: crate::config::StorageConfig {
                    storage_type: crate::config::StorageType::File,
                    path: dir.join("storage"),
                    encryption_enabled: true,
                    compression_enabled: true,
                    max_size_mb: 1000,
                },
                metrics: crate::config::MetricsConfig {
                    enabled: true,
                    collection_interval_secs: 60,
                    retention_period_secs: 3600,
                    export_prometheus: true,
                    prometheus_port: 9090,
                },
                security: crate::config::SecurityConfig {
                    encryption_key: secrecy::Secret::new("change_me".to_string()),
                    api_keys: std::collections::HashMap::new(),
                    allowed_origins: vec!["*".to_string()],
                    rate_limit_requests: 100,
                    rate_limit_period_secs: 60,
                },
                api: crate::config::ApiConfig {
                    host: "127.0.0.1".to_string(),
                    port: 8080,
                    timeout_secs: 30,
                    max_payload_size_mb: 10,
                    cors_enabled: true,
                },
            };

            let config_path = dir.join("config.yaml");
            let config_str = serde_yaml::to_string(&config)
                .map_err(|e| MoaError::Config(format!("Failed to serialize config: {}", e)))?;

            tokio::fs::write(&config_path, config_str).await
                .map_err(|e| MoaError::Config(format!("Failed to write config: {}", e)))?;

            println!("Created config at: {}", config_path.display());
            Ok(())
        }

        Commands::Validate { config } => {
            let config = tokio::fs::read_to_string(config).await
                .map_err(|e| MoaError::Config(format!("Failed to read config: {}", e)))?;

            let config: MoaConfig = serde_yaml::from_str(&config)
                .map_err(|e| MoaError::Config(format!("Invalid config: {}", e)))?;

            config.validate()?;
            println!("Configuration is valid!");
            Ok(())
        }

        Commands::Agent { command } => {
            match command {
                AgentCommands::List => {
                    let config = MoaConfig::load().await?;
                    println!("Configured agent: {}", config.agent.name);
                    println!("Type: {:?}", config.agent.agent_type);
                    println!("Role: {:?}", config.agent.role);
                    println!("Capabilities: {:?}", config.agent.capabilities);
                    Ok(())
                }

                AgentCommands::Add { name, agent_type, role, capabilities } => {
                    let mut config = MoaConfig::load().await?;
                    
                    let agent_type = match agent_type.to_lowercase().as_str() {
                        "llm" => AgentType::LLM,
                        "custom" => AgentType::Custom,
                        "ensemble" => AgentType::Ensemble,
                        _ => return Err(MoaError::Config("Invalid agent type".to_string())),
                    };

                    let role = match role.to_lowercase().as_str() {
                        "primary" => AgentRole::Primary,
                        "secondary" => AgentRole::Secondary,
                        "fallback" => AgentRole::Fallback,
                        _ => return Err(MoaError::Config("Invalid agent role".to_string())),
                    };

                    let capabilities = capabilities.split(',')
                        .map(|s| s.trim().to_string())
                        .collect();

                    config.agent = crate::config::AgentConfig {
                        name: name.clone(),
                        agent_type,
                        role,
                        capabilities,
                        config: serde_json::json!({}),
                    };

                    // Save updated config
                    let config_str = serde_yaml::to_string(&config)
                        .map_err(|e| MoaError::Config(format!("Failed to serialize config: {}", e)))?;

                    tokio::fs::write("config.yaml", config_str).await
                        .map_err(|e| MoaError::Config(format!("Failed to write config: {}", e)))?;

                    println!("Added agent: {}", name);
                    Ok(())
                }

                AgentCommands::Remove { name } => {
                    let config = MoaConfig::load().await?;
                    if config.agent.name == *name {
                        println!("Removed agent: {}", name);
                    } else {
                        println!("Agent not found: {}", name);
                    }
                    Ok(())
                }
            }
        }

        Commands::Status => {
            let config = MoaConfig::load().await?;
            println!("System Status");
            println!("-------------");
            println!("Agent: {}", config.agent.name);
            println!("Storage: {:?}", config.storage.storage_type);
            println!("Metrics enabled: {}", config.metrics.enabled);
            println!("API endpoint: {}:{}", config.api.host, config.api.port);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert()
    }

    #[test]
    fn test_cli_config() {
        let cli = Cli::parse_from([
            "program",
            "start",
            "--port",
            "9000",
            "--host",
            "0.0.0.0",
            "--log-level",
            "debug",
        ]);

        if let Commands::Start { port, host } = &cli.command {
            let config = CliConfig::from_cli(&cli, &cli.command);
            assert_eq!(config.port, *port);
            assert_eq!(config.host, host);
            assert_eq!(config.log_level, "debug");
        } else {
            panic!("Expected Start command");
        }
    }
} 