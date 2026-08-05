use clap::{Args, Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tiwut_ai_v2::api::{ApiServer, AppState};
use tiwut_ai_v2::chat::ChatEngine;
use tiwut_ai_v2::config::AppConfig;
use tiwut_ai_v2::hardware::HardwareManager;
use tiwut_ai_v2::memory::MemoryBank;
use tiwut_ai_v2::model::TiwutModel;
use tiwut_ai_v2::package::ModelPackage;
use tiwut_ai_v2::tokenizer::Tokenizer;
use tiwut_ai_v2::trainer::NeuralTrainer;

#[derive(Parser)]
#[command(name = "tiwut-ai")]
#[command(author = "Tiwut")]
#[command(version = "2.0.0")]
#[command(about = "High-Performance Cross-Platform Neural Network AI Engine & API Server", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short = 'c', long = "chat")]
    chat: bool,

    #[arg(short = 'a', long = "ask")]
    ask: Option<String>,

    #[arg(short = 't', long = "train")]
    train: bool,

    #[arg(long = "url")]
    url: Option<String>,

    #[arg(long = "file")]
    file: Option<PathBuf>,

    #[arg(long = "dir")]
    dir: Option<PathBuf>,

    #[arg(short = 'e', long = "epochs")]
    epochs: Option<usize>,

    #[arg(long = "lr")]
    lr: Option<f32>,

    #[arg(long = "default-knowledge")]
    default_knowledge: bool,

    #[arg(short = 's', long = "serve")]
    serve: bool,

    #[arg(short = 'p', long = "port", default_value_t = 8080)]
    port: u16,

    #[arg(long = "host", default_value = "127.0.0.1")]
    host: String,

    #[arg(short = 'm', long = "model", default_value = "ai.model")]
    model_path: String,

    #[arg(long = "status")]
    status: bool,

    #[arg(long = "reset")]
    reset: bool,
}

#[derive(Subcommand)]
enum Commands {

    Chat,

    Ask { question: String },

    Train(TrainArgs),

    Serve(ServeArgs),

    Status,

    Config,

    Reset,

    InitDefault,
}

#[derive(Args)]
struct TrainArgs {
    #[arg(long = "url")]
    url: Option<String>,

    #[arg(long = "file")]
    file: Option<PathBuf>,

    #[arg(long = "dir")]
    dir: Option<PathBuf>,

    #[arg(long = "text")]
    text: Option<String>,

    #[arg(short = 'e', long = "epochs")]
    epochs: Option<usize>,

    #[arg(long = "lr")]
    lr: Option<f32>,

    #[arg(long = "default-knowledge", default_value_t = false)]
    default_knowledge: bool,
}

#[derive(Args)]
struct ServeArgs {
    #[arg(short = 'p', long = "port", default_value_t = 8080)]
    port: u16,

    #[arg(long = "host", default_value = "127.0.0.1")]
    host: String,
}

fn load_or_init_model(model_path: &str) -> (AppConfig, TiwutModel, Tokenizer, MemoryBank) {
    if ModelPackage::exists(model_path) {
        match ModelPackage::load_from_file(model_path) {
            Ok(state) => return state,
            Err(e) => {
                eprintln!(
                    "{} Failed to load existing model package '{}': {}. Re-initializing...",
                    "⚠️".yellow(),
                    model_path,
                    e
                );
            }
        }
    }

    println!(
        "{} Initializing fresh Tiwut-AI v2 model with built-in English knowledge base...",
        "✨".bright_cyan()
    );

    let mut config = AppConfig::default();
    let mut tokenizer = Tokenizer::default();
    let mut model = TiwutModel::new(config.model.clone());
    let mut memory = MemoryBank::new();

    let default_lr = config.training.learning_rate;
    let _ = NeuralTrainer::train_sources(
        &mut config,
        &mut model,
        &mut tokenizer,
        &mut memory,
        None,
        None,
        None,
        None,
        true,
        Some(6),
        Some(default_lr),
        model_path,
        |evt| {
            if evt.stage == "training" {
                println!("  {}", evt.message.dimmed());
            }
        },
    );

    let _ = ModelPackage::save_to_file(model_path, &config, &model, &tokenizer, &memory);
    println!(
        "{} Saved initialized model to '{}'\n",
        "✓".bright_green(),
        model_path
    );

    (config, model, tokenizer, memory)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let model_path = cli.model_path.clone();

    if cli.status || matches!(cli.command, Some(Commands::Status)) {
        let (config, model, tokenizer, memory) = load_or_init_model(&model_path);
        let hw = HardwareManager::get_info();
        println!("\n{}", "Neural Network & System Telemetry:".bold().bright_cyan());
        println!("  • Engine:           Tiwut-AI v2 (Native Rust Engine)");
        println!("  • OS & Arch:        {} ({})", hw.os, hw.arch);
        println!("  • Chip:             {}", hw.chip_name);
        println!("  • CPU Cores:        {}", hw.cpu_cores);
        println!("  • Acceleration:     {}", hw.acceleration_engine);
        println!("  • System RAM:       {} MB total ({} MB available)", hw.total_ram_mb, hw.available_ram_mb);
        println!("  • Model Package:    {}", model_path.bright_yellow());
        println!("  • Model Parameters: {:?} parameters ({} layers, {} heads)", model.total_parameters(), config.model.num_layers, config.model.num_heads);
        println!("  • Vocab Size:       {} tokens", tokenizer.vocab_size());
        println!("  • Memory Bank:      {:.2} MB in RAM ({} chunks, {} tokens)", memory.memory_usage_mb(), memory.chunks.len(), memory.metadata.total_tokens);
        println!("  • Learned Sources:  {} document(s)\n", memory.metadata.sources.len());
        return Ok(());
    }

    if cli.reset || matches!(cli.command, Some(Commands::Reset)) {
        let config = AppConfig::default();
        let tokenizer = Tokenizer::default();
        let model = TiwutModel::new(config.model.clone());
        let memory = MemoryBank::new();

        ModelPackage::save_to_file(&model_path, &config, &model, &tokenizer, &memory)?;
        println!(
            "\n{} Successfully reset neural model, vocabulary, and knowledge in '{}'.\n",
            "🧹".bright_green(),
            model_path
        );
        return Ok(());
    }

    if matches!(cli.command, Some(Commands::Config)) {
        let (config, _, _, _) = load_or_init_model(&model_path);
        let json = serde_json::to_string_pretty(&config)?;
        println!("\n{}\n{}\n", "Active Configuration:".bold().bright_cyan(), json);
        return Ok(());
    }

    if matches!(cli.command, Some(Commands::InitDefault)) {
        let mut config = AppConfig::default();
        let mut tokenizer = Tokenizer::default();
        let mut model = TiwutModel::new(config.model.clone());
        let mut memory = MemoryBank::new();

        let lr = config.training.learning_rate;
        println!("{}", "🧠 Initializing and training on Built-in English Knowledge...".bright_cyan().bold());
        NeuralTrainer::train_sources(
            &mut config,
            &mut model,
            &mut tokenizer,
            &mut memory,
            None,
            None,
            None,
            None,
            true,
            Some(8),
            Some(lr),
            &model_path,
            |evt| println!("  {}", evt.message),
        )?;
        return Ok(());
    }

    if cli.train || matches!(cli.command, Some(Commands::Train(_))) {
        let (mut config, mut model, mut tokenizer, mut memory) = load_or_init_model(&model_path);

        let (urls, files, dirs, texts, epochs, lr, def_know) = match cli.command {
            Some(Commands::Train(args)) => {
                let urls = args.url.map(|u| vec![u]);
                let files = args.file.map(|f| vec![f]);
                let dirs = args.dir.map(|d| vec![d]);
                let texts = args.text.map(|t| vec![t]);
                (urls, files, dirs, texts, args.epochs, args.lr, args.default_knowledge)
            }
            _ => {
                let urls = cli.url.map(|u| vec![u]);
                let files = cli.file.map(|f| vec![f]);
                let dirs = cli.dir.map(|d| vec![d]);
                (urls, files, dirs, None, cli.epochs, cli.lr, cli.default_knowledge)
            }
        };

        println!("\n{}", "🧠 Starting Neural Training Pipeline...".bold().bright_cyan());
        NeuralTrainer::train_sources(
            &mut config,
            &mut model,
            &mut tokenizer,
            &mut memory,
            urls.as_deref(),
            files.as_deref(),
            dirs.as_deref(),
            texts.as_deref(),
            def_know,
            epochs,
            lr,
            &model_path,
            |evt| {
                if evt.stage == "training" {
                    println!("  {}", evt.message.bright_yellow());
                } else if evt.stage == "complete" {
                    println!("\n{}", evt.message.bright_green().bold());
                } else {
                    println!("  {}", evt.message);
                }
            },
        )?;
        return Ok(());
    }

    if let Some(q) = cli.ask {
        let (config, model, tokenizer, memory) = load_or_init_model(&model_path);
        println!("\n{} {}", "Question:".bold().bright_green(), q);
        print!("{} ", "Answer:".bold().bright_cyan());
        let _ = ChatEngine::answer_query(&q, &config, &model, &tokenizer, &memory, true, None);
        println!();
        return Ok(());
    }
    if let Some(Commands::Ask { question }) = cli.command {
        let (config, model, tokenizer, memory) = load_or_init_model(&model_path);
        println!("\n{} {}", "Question:".bold().bright_green(), question);
        print!("{} ", "Answer:".bold().bright_cyan());
        let _ = ChatEngine::answer_query(&question, &config, &model, &tokenizer, &memory, true, None);
        println!();
        return Ok(());
    }

    if cli.serve || matches!(cli.command, Some(Commands::Serve(_))) {
        let (host, port) = match cli.command {
            Some(Commands::Serve(args)) => (args.host, args.port),
            _ => (cli.host, cli.port),
        };

        let (config, model, tokenizer, memory) = load_or_init_model(&model_path);
        let state = Arc::new(AppState {
            config: Mutex::new(config),
            model: Mutex::new(model),
            tokenizer: Mutex::new(tokenizer),
            memory: Mutex::new(memory),
            model_path,
        });

        ApiServer::start(state, &host, port).await?;
        return Ok(());
    }

    let (config, model, tokenizer, memory) = load_or_init_model(&model_path);
    ChatEngine::start_interactive_session(&config, &model, &tokenizer, &memory);
    Ok(())
}

