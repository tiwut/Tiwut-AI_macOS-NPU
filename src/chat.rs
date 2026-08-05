use crate::config::AppConfig;
use crate::hardware::HardwareManager;
use crate::memory::MemoryBank;
use crate::model::TiwutModel;
use crate::tokenizer::Tokenizer;
use colored::Colorize;
use std::io::{self, Write};
use std::time::Instant;

pub struct ChatEngine;

impl ChatEngine {
    pub fn answer_query(
        user_query: &str,
        config: &AppConfig,
        model: &TiwutModel,
        tokenizer: &Tokenizer,
        memory: &MemoryBank,
        stream: bool,
        mut token_cb: Option<&mut dyn FnMut(&str)>,
    ) -> String {
        let q = user_query.trim();
        if q.is_empty() {
            return String::new();
        }

        let q_lower = q.to_lowercase();

        let q_clean: String = q_lower
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == ' ')
            .collect::<String>()
            .trim()
            .to_string();

        if matches!(
            q_clean.as_str(),
            "hi" | "hello" | "hey" | "yo" | "greetings" | "good morning" | "good evening" | "good afternoon"
        ) || q_clean.starts_with("hello ")
            || q_clean.starts_with("hi ")
            || q_clean.starts_with("hey ")
        {
            let reply = "Hello! I am Tiwut-AI, your high-performance neural assistant. How can I help you today?";
            Self::output_text(reply, stream, &mut token_cb);
            return reply.to_string();
        }

        if q_lower.contains("who are you")
            || q_lower.contains("what are you")
            || q_lower.contains("your name")
            || q_lower == "what is tiwut-ai"
            || q_lower == "what is tiwut"
        {
            let hw = HardwareManager::get_info();
            let reply = format!(
                "I am Tiwut-AI Version 2, a fast and efficient cross-platform neural network AI engine built in Rust.\nRunning on {} with {}.\nI currently hold {} knowledge chunks in memory across {} source(s).",
                hw.chip_name,
                hw.acceleration_engine,
                memory.chunks.len(),
                memory.metadata.sources.len()
            );
            Self::output_text(&reply, stream, &mut token_cb);
            return reply;
        }

        let query_tokens = tokenizer.encode(q, false);
        let query_vec = model.encode_semantic_vector(&query_tokens);

        let search_results = memory.search(&query_vec, q, 6, config.inference.memory_threshold);

        if !search_results.is_empty() {
            if let Some(extracted) = memory.extract_intelligent_answer(q, &search_results) {
                let source_ref = &search_results[0].chunk.source;
                let reply = format!("{}\n\n[Source: {}]", extracted, source_ref);
                Self::output_text(&reply, stream, &mut token_cb);
                return reply;
            }
        }

        let prompt = format!("User: {}\nAssistant:", q);
        let mut input_tokens = tokenizer.encode(&prompt, true);

        let max_tokens = config.inference.max_tokens;
        let temp = config.inference.temperature;
        let top_k = config.inference.top_k;
        let top_p = config.inference.top_p;
        let rep_pen = config.inference.repetition_penalty;

        let mut generated_text = String::new();

        for _ in 0..max_tokens {
            let next_token = model.generate_next_token(&input_tokens, temp, top_k, top_p, rep_pen);

            if next_token == crate::tokenizer::EOS_TOKEN_ID {
                break;
            }

            let token_str = tokenizer.decode(&[next_token], true);
            generated_text.push_str(&token_str);

            if let Some(ref mut cb) = token_cb {
                cb(&token_str);
            } else if stream {
                print!("{}", token_str);
                let _ = io::stdout().flush();
            }

            input_tokens.push(next_token);
        }

        if stream && token_cb.is_none() {
            println!();
        }

        let trimmed = generated_text.trim();
        if trimmed.is_empty() {
            let fallback = "I don't have enough knowledge on that topic in my memory yet. You can train me using: tiwut-ai train -url <URL> or -file <path>.";
            Self::output_text(fallback, stream, &mut token_cb);
            return fallback.to_string();
        }

        trimmed.to_string()
    }

    fn output_text(text: &str, stream: bool, cb: &mut Option<&mut dyn FnMut(&str)>) {
        if let Some(ref mut callback) = cb {
            callback(text);
        } else if stream {
            for word in text.split(' ') {
                print!("{} ", word);
                let _ = io::stdout().flush();
                std::thread::sleep(std::time::Duration::from_millis(15));
            }
            println!();
        } else {
            println!("{}", text);
        }
    }

    pub fn start_interactive_session(
        config: &AppConfig,
        model: &TiwutModel,
        tokenizer: &Tokenizer,
        memory: &MemoryBank,
    ) {
        let hw = HardwareManager::get_info();
        println!("\n{}", "=".repeat(72).cyan());
        println!(
            " {}",
            "🤖 Tiwut-AI Version 2 (Rust High-Performance Neural Engine)".bold().bright_magenta()
        );
        println!(
            " {}",
            format!(
                "⚡ Hardware: {} | Cores: {} | RAM Memory Bank: {:.2} MB ({} chunks)",
                hw.chip_name,
                hw.cpu_cores,
                memory.memory_usage_mb(),
                memory.chunks.len()
            )
            .dimmed()
        );
        println!(
            " {}",
            "💡 Commands: /help, /status, /memory, /clear, /exit".dimmed()
        );
        println!("{}\n", "=".repeat(72).cyan());

        let mut history: Vec<(String, String)> = Vec::new();

        loop {
            print!("{}", "You > ".bright_green().bold());
            let _ = io::stdout().flush();

            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_err() {
                break;
            }

            let input_trim = input.trim();
            if input_trim.is_empty() {
                continue;
            }

            match input_trim.to_lowercase().as_str() {
                "/exit" | "/quit" | "exit" | "quit" | ":q" => {
                    println!("\n{} 👋 Exiting Tiwut-AI v2. Have a great day!\n", "Tiwut-AI:".cyan().bold());
                    break;
                }
                "/help" => {
                    println!("\n{}", "Chat Commands:".bold());
                    println!("  /help    - Show this help message");
                    println!("  /status  - Display neural parameters, hardware telemetry & RAM stats");
                    println!("  /memory  - List ingested sources and knowledge chunks");
                    println!("  /clear   - Clear conversation history");
                    println!("  /exit    - Exit interactive chat\n");
                    continue;
                }
                "/status" => {
                    println!("\n{}", "Neural Network & Hardware Telemetry:".bold().cyan());
                    println!("  • Engine:           Tiwut-AI v2 (Native Rust)");
                    println!("  • Platform:         {} ({})", hw.os, hw.arch);
                    println!("  • Processor:        {}", hw.chip_name);
                    println!("  • Cores / Threads:  {}", hw.cpu_cores);
                    println!("  • Acceleration:     {}", hw.acceleration_engine);
                    println!("  • Total RAM:        {} MB (Available: {} MB)", hw.total_ram_mb, hw.available_ram_mb);
                    println!("  • Model Parameters: {:?} parameters", model.total_parameters());
                    println!("  • Vocab Size:       {} tokens", tokenizer.vocab_size());
                    println!("  • In-RAM Memory:    {:.2} MB ({} chunks, {} tokens)", memory.memory_usage_mb(), memory.chunks.len(), memory.metadata.total_tokens);
                    println!("  • Learned Sources:  {} document(s)\n", memory.metadata.sources.len());
                    continue;
                }
                "/memory" => {
                    println!("\n{}", format!("In-RAM Knowledge Base ({} chunks):", memory.chunks.len()).bold());
                    if memory.metadata.sources.is_empty() {
                        println!("  (No custom documents trained yet. Train with 'tiwut-ai train -url <URL>')\n");
                    } else {
                        for (i, src) in memory.metadata.sources.iter().enumerate() {
                            println!("  {}. {}", i + 1, src.bright_yellow());
                        }
                        println!();
                    }
                    continue;
                }
                "/clear" => {
                    history.clear();
                    println!("\n{}\n", "🧹 Conversation history cleared.".bright_yellow());
                    continue;
                }
                _ => {}
            }

            print!("\n{} ", "Tiwut-AI >".bright_cyan().bold());
            let _ = io::stdout().flush();

            let t0 = Instant::now();
            let response = Self::answer_query(input_trim, config, model, tokenizer, memory, true, None);
            let elapsed = t0.elapsed().as_secs_f32();

            println!(
                "{}",
                format!("\n[Inference: {:.3}s | Multi-threaded Rayon SIMD]\n", elapsed).dimmed()
            );

            history.push((input_trim.to_string(), response));
        }
    }
}

