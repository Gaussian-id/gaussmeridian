use clap::{Parser, Subcommand, Args};
use gaussmoa::{
    config::{self, AgentConfig, AgentType},
    error::MoaError,
    agents::{self, LlmAgentConfig, LlmProvider, llm_agent::RetryConfig},
    models::MoaRequest,
    providers::{ChatProvider, HttpChatProvider},
    MoaEngine,
    MoaResult,
};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tracing::{error, info, warn, Level};
use tracing_subscriber::{self, fmt::format::FmtSpan};
use tokio::time::Duration;
use futures::{stream, StreamExt};
use serde_json::json;
use std::time::Instant;
use uuid::Uuid;
use chrono::Utc;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Query(QueryArgs),
    Interactive(InteractiveArgs),
    Benchmark(BenchmarkArgs),
    Agent(AgentCommand),
}

#[derive(Args)]
struct QueryArgs {
    query: String,
    #[arg(short, long)]
    context: Option<String>,
    #[arg(short, long, default_value = "2048")]
    max_tokens: usize,
    #[arg(short, long, default_value = "0.7")]
    temperature: f32,
}

#[derive(Args)]
struct InteractiveArgs {
    #[arg(short, long)]
    completion: bool,
    #[arg(short, long, default_value = "~/.moa_history")]
    history: PathBuf,
    #[arg(short, long, default_value = "2048")]
    max_tokens: usize,
}

#[derive(Args)]
struct BenchmarkArgs {
    #[arg(short, long, default_value = "100")]
    queries: usize,
    #[arg(short, long, default_value = "10")]
    concurrent: usize,
    #[arg(short, long)]
    json: bool,
}

#[derive(Args)]
struct AgentCommand {
    #[command(subcommand)]
    command: AgentSubCommand,
}

#[derive(Subcommand)]
enum AgentSubCommand {
    List,
    Add {
        name: String,
        #[arg(short, long)]
        model: String,
        #[arg(short, long, default_value = "primary")]
        role: String,
        #[arg(long, default_value = "0.7")]
        temperature: f32,
        #[arg(long, default_value = "2048")]
        max_tokens: usize,
    },
    Remove {
        name: String,
    },
}

#[tokio::main]
async fn main() -> MoaResult<()> {
    let cli = Cli::parse();
    let log_level = match cli.verbose {
        0 => Level::WARN, 1 => Level::INFO, 2 => Level::DEBUG, _ => Level::TRACE,
    };
    tracing_subscriber::fmt().with_max_level(log_level).with_span_events(FmtSpan::CLOSE).with_target(false).init();

    let config_path = cli.config.unwrap_or_else(|| PathBuf::from("config.toml"));
    
    // Standalone/debug mode calls providers via the built-in OpenAI-compatible client
    // (env OPENAI_API_KEY / OPENAI_API_BASE). The gateway injects its own provider instead.
    let provider: Arc<dyn ChatProvider> = Arc::new(HttpChatProvider::from_env());

    let mut engine = MoaEngine::new(config_path.clone()).await?;

    if engine.is_empty().await {
        info!("No agents in config, adding default OpenAI agent.");
        let default_llm_config = LlmAgentConfig {
            provider: LlmProvider::OpenAI { model: "gpt-4o-mini".to_string(), temperature: 0.7, max_tokens: 2048 },
            system_prompt: Some("You are a helpful AI assistant.".to_string()),
            response_format: None,
            timeout_secs: 30,
            retries: Some(RetryConfig {
                max_retries: 3,
                initial_delay_ms: 100,
                max_delay_ms: 5000,
                backoff_factor: 2.0,
            }),
        };
        
        let agent_specific_config_json = serde_json::to_value(&default_llm_config).map_err(MoaError::from)?;

        let agent_name = "default_openai_agent".to_string();
        let parsed_role = config::AgentRole::Primary;
        let default_capabilities = vec!["text_generation".to_string()];

        let agent_config_for_engine = AgentConfig {
            name: agent_name.clone(),
            agent_type: AgentType::LLM,
            role: parsed_role,
            capabilities: default_capabilities.clone(),
            config: agent_specific_config_json,
            max_retries: 3,
            timeout_secs: 30,
        };
        
        let agent = Box::new(agents::LlmAgent::new(
            agent_config_for_engine.name.clone(),
            agent_config_for_engine.role.clone(),
            default_llm_config,
            provider.clone(),
        ));
        engine.add_agent(agent).await?;
        info!("Default agent 'gpt-4o-mini' added.");
    } else {
        let current_agents = engine.list_agents().await; 
        info!("Engine initialized with {} agent(s) from configuration.", current_agents.len());
    }
    
    match cli.command {
        Commands::Query(args) => process_query(&engine, args).await?,
        Commands::Interactive(args) => run_interactive_mode(&mut engine, args, provider.clone()).await?,
        Commands::Benchmark(args) => run_benchmark(&engine, args).await?,
        Commands::Agent(agent_cmd) => handle_agent_command(&mut engine, agent_cmd.command, provider.clone()).await?,
    }
    Ok(())
}

fn create_moa_request_with_metadata(query: String, context: Option<String>, max_tokens: Option<usize>, temperature: Option<f32>) -> MoaRequest {
    let mut metadata = HashMap::new();
    if let Some(mt) = max_tokens { metadata.insert("max_tokens".to_string(), mt.to_string()); }
    if let Some(temp) = temperature { metadata.insert("temperature".to_string(), temp.to_string()); }
    
    MoaRequest {
        id: Uuid::new_v4().to_string(),
        query,
        context,
        timestamp: Utc::now(),
        metadata,
    }
}

async fn process_query(engine: &MoaEngine, args: QueryArgs) -> MoaResult<()> {
    let start = Instant::now();
    let _moa_request = create_moa_request_with_metadata(args.query.clone(), args.context.clone(), Some(args.max_tokens), Some(args.temperature));
    
    let response = engine.process_query(&args.query, args.context.as_deref()).await?;
    let duration = start.elapsed();
    println!("\nResponse: {}", response.content);
    println!("Confidence: {:.2}", response.confidence);
    println!("Time taken: {:.2}s", duration.as_secs_f32());
    Ok(())
}

async fn run_interactive_mode(engine: &mut MoaEngine, args: InteractiveArgs, provider: Arc<dyn ChatProvider>) -> MoaResult<()> {
    use rustyline::Editor;
    use rustyline::validate::MatchingBracketValidator;
    use rustyline_derive::{Completer, Helper, Highlighter, Hinter, Validator};
    #[derive(Completer, Helper, Highlighter, Hinter, Validator)]
    struct InputValidator { #[rustyline(Validator)] brackets: MatchingBracketValidator, }

    println!("MoA-RS Interactive Mode (Powered by GaussMOA)");
    println!("Type 'help' for commands, 'list_agents', 'add_agent', 'remove_agent <name>', 'exit' to quit");
    println!();
    
    let mut rl = Editor::<InputValidator, _>::new()?;
    rl.set_helper(Some(InputValidator { brackets: MatchingBracketValidator::new() }));
    let history_path_str = args.history.to_string_lossy();
    let expanded_history_path = shellexpand::tilde(&history_path_str).into_owned();
    let history_path = PathBuf::from(expanded_history_path);
    if history_path.exists() { if let Err(e) = rl.load_history(&history_path) { warn!("Failed to load history: {}", e); } }
    else if let Some(pd) = history_path.parent() { if !pd.exists() { if let Err(e) = std::fs::create_dir_all(pd) { warn!("Failed to create history dir: {}", e);}} }


    loop {
        match rl.readline("moa> ") {
            Ok(line) => {
                let line_trimmed = line.trim();
                if line_trimmed.is_empty() { continue; }
                rl.add_history_entry(line_trimmed)?;
                let parts: Vec<&str> = line_trimmed.split_whitespace().collect();
                match parts[0] {
                    "exit" | "quit" => break,
                    "help" => {
                        println!("Commands:");
                        println!("  help                - Show this help");
                        println!("  exit                - Exit the program");
                        println!("  clear               - Clear the screen");
                        println!("  history             - Show command history");
                        println!("  list_agents         - List configured agents");
                        println!("  add_agent <name> --model <model_id> [--role <role>] [--temp <float>] [--max_tokens <int>] - Add LLM agent");
                        println!("  remove_agent <name> - Remove an agent");
                        println!("  Any other text will be processed as a query.");
                    }
                    "clear" => print!("\x1B[2J\x1B[1;1H"),
                    "history" => { for (i, hist_item) in rl.history().iter().enumerate() { println!("{}: {}", i + 1, hist_item); } }
                    "list_agents" => {
                        let agents_list = engine.list_agents().await;
                        if agents_list.is_empty() {
                            println!("No agents configured.");
                        } else {
                            println!("Configured agents:");
                            for (name, role, metric) in agents_list {
                                println!("- Name: {}, Role: {:?}, Metric: {:.2}", name, role, metric);
                            }
                        }
                    }
                    "add_agent" => {
                        if parts.len() >= 3 {
                            let name = parts[1].to_string();
                            let model_id = parts.iter().skip_while(|p| **p != "--model").nth(1).map(|s| *s).unwrap_or("gpt-4o-mini").to_string();
                            let role_str = parts.iter().skip_while(|p| **p != "--role").nth(1).map(|s| *s).unwrap_or("primary").to_string();
                            let temp:f32 = parts.iter().skip_while(|p|**p != "--temp").nth(1).and_then(|s| (*s).parse().ok()).unwrap_or(0.7);
                            let max_tok:usize = parts.iter().skip_while(|p|**p != "--max_tokens").nth(1).and_then(|s| (*s).parse().ok()).unwrap_or(2048);

                            let agent_role = match role_str.to_lowercase().as_str() {
                                "primary" => config::AgentRole::Primary,
                                "secondary" => config::AgentRole::Secondary,
                                "fallback" => config::AgentRole::Fallback,
                                _ => { println!("Invalid role: {}. Use primary, secondary, or fallback.", role_str); continue; }
                            };
                            let llm_conf = LlmAgentConfig { provider: LlmProvider::OpenAI { model: model_id, temperature: temp, max_tokens: max_tok }, system_prompt: Some("You are a helpful AI assistant.".to_string()), response_format: None, timeout_secs: 60, retries: None };
                            let new_agent = Box::new(agents::LlmAgent::new(name.clone(), agent_role, llm_conf, provider.clone()));
                            if let Err(e) = engine.add_agent(new_agent).await {
                                error!("Failed to add agent '{}': {}", name, e);
                            } else {
                                info!("Agent '{}' added.", name);
                            }
                        } else { println!("Usage: add_agent <name> --model <id> ..."); }
                    }
                    "remove_agent" => {
                        if parts.len()==2 { 
                            let agent_name_to_remove = parts[1].to_string();
                            if let Err(e)=engine.remove_agent(&agent_name_to_remove).await {error!("Failed to remove '{}': {}",agent_name_to_remove, e);} 
                            else {info!("Agent '{}' removed.", agent_name_to_remove);}
                        }
                        else {println!("Usage: remove_agent <name>");}
                    }
                    _ => {
                        let query_args = QueryArgs { query: line_trimmed.to_string(), context: None, max_tokens: args.max_tokens, temperature: 0.7 };
                        if let Err(e) = process_query(engine, query_args).await { error!("Query error: {}", e); }
                    }
                }
            }
            Err(rustyline::error::ReadlineError::Interrupted) => { println!("CTRL-C"); break; }
            Err(rustyline::error::ReadlineError::Eof) => { println!("CTRL-D"); break; }
            Err(err) => { error!("Readline error: {}", err); break; }
        }
    }
    if let Err(e) = rl.save_history(&history_path) { warn!("Failed to save history: {}", e); }
    Ok(())
}

async fn run_benchmark(engine: &MoaEngine, args: BenchmarkArgs) -> MoaResult<()> {
    info!("Benchmark: {} queries, {} concurrent tasks", args.queries, args.concurrent);
    let overall_start_time = Instant::now();
    let benchmark_queries_list = vec![
        "What is the capital of France?", "Explain relativity.", "Causes of climate change?",
        "Short story about a robot.", "Best way to learn Rust?", "Summarize Hamlet."
    ];

    println!("Running benchmark ({} queries, {} concurrent)...", args.queries, args.concurrent);

    let mut stream = stream::iter(0..args.queries)
        .map(|i| {
            let query_text = benchmark_queries_list[i % benchmark_queries_list.len()].to_string();
            async move {
                let query_start_time = Instant::now();
                let result = engine.process_query(&query_text, None).await;
                let duration = query_start_time.elapsed();
                (query_text, result, duration)
            }
        })
        .buffered(args.concurrent);

    let mut results_data = Vec::new();
    let mut successful_queries = 0; let mut failed_queries = 0;
    let mut total_query_processing_time = Duration::new(0,0);

    while let Some((query, result, duration)) = stream.next().await {
        total_query_processing_time += duration;
        match result {
            Ok(response) => { successful_queries += 1; results_data.push(json!({ "query": query, "success": true, "duration_ms": duration.as_millis(), "confidence": response.confidence })); }
            Err(e) => { failed_queries += 1; results_data.push(json!({ "query": query, "success": false, "error": e.to_string(), "duration_ms": duration.as_millis() })); error!("Benchmark query '{}' failed: {}", query, e); }
        }
    }
    let overall_duration = overall_start_time.elapsed();
    let avg_query_time_ms = if successful_queries + failed_queries > 0 { (total_query_processing_time.as_millis() as f64) / ((successful_queries + failed_queries) as f64) } else {0.0};
    
    let report = json!({
        "total_queries_run": args.queries,
        "concurrent_tasks": args.concurrent,
        "successful_queries": successful_queries,
        "failed_queries": failed_queries,
        "total_benchmark_time_secs": overall_duration.as_secs_f64(),
        "total_query_processing_time_secs": total_query_processing_time.as_secs_f64(),        
        "average_query_time_ms": avg_query_time_ms,
        "queries_per_second": if overall_duration.as_secs_f64() > 0.0 { (args.queries as f64) / overall_duration.as_secs_f64() } else { 0.0 },
        "results": if args.json { serde_json::Value::Array(results_data) } else { serde_json::Value::Null },
    });

    if args.json { println!("\n{}", serde_json::to_string_pretty(&report)?); }
    else { println!("\n--- Benchmark Summary ---\nQueries: {}, Success: {}, Fail: {}, Concurrency: {}, Total Time: {:.2}s, Avg Query: {:.2}ms, QPS: {:.2}", args.queries, successful_queries, failed_queries, args.concurrent, overall_duration.as_secs_f64(), avg_query_time_ms, if overall_duration.as_secs_f64() > 0.0 { (args.queries as f64) / overall_duration.as_secs_f64() } else {0.0}); }
    Ok(())
}

async fn handle_agent_command(engine: &mut MoaEngine, command: AgentSubCommand, provider: Arc<dyn ChatProvider>) -> MoaResult<()> {
    match command {
        AgentSubCommand::List => {
            let agents_list = engine.list_agents().await;
            if agents_list.is_empty() {
                println!("No agents configured.");
            } else {
                println!("Configured agents:");
                for (name, role, metric) in agents_list {
                    println!("- Name: {}, Role: {:?}, Metric: {:.2}", name, role, metric);
                }
            }
        }
        AgentSubCommand::Add { name, model, role, temperature, max_tokens } => {
            let agent_role = match role.to_lowercase().as_str() {
                "primary" => config::AgentRole::Primary,
                "secondary" => config::AgentRole::Secondary,
                "fallback" => config::AgentRole::Fallback,
                _ => return Err(MoaError::config(format!("Invalid agent role: '{}'. Use primary, secondary, or fallback.", role), None::<Box<dyn std::error::Error + Send + Sync>>)),
            };
            let llm_specific_config = LlmAgentConfig { 
                provider: LlmProvider::OpenAI { model, temperature, max_tokens }, 
                system_prompt: Some("You are a helpful AI assistant.".to_string()), 
                response_format: None, timeout_secs: 60, retries: None 
            };
            let agent = Box::new(agents::LlmAgent::new(name.clone(), agent_role, llm_specific_config, provider));
            engine.add_agent(agent).await?;
            info!("Agent '{}' added.", name);
        }
        AgentSubCommand::Remove { name } => {
            engine.remove_agent(&name).await?;
            info!("Agent '{}' removed.", name);
        }
    }
    Ok(())
}

trait WithMetadata {
    fn with_metadata(self, metadata: HashMap<String, String>) -> Self;
}

impl WithMetadata for MoaRequest {
    fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }
}