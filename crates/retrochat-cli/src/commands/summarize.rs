use anyhow::{Context, Result};
use std::sync::Arc;
use uuid::Uuid;

use retrochat_core::database::DatabaseManager;
use retrochat_core::env::{apis as env_apis, llm as env_llm};
use retrochat_core::services::llm::{LlmClientFactory, LlmConfig, LlmProvider};
use retrochat_core::services::{SessionSummarizer, TurnDetector, TurnSummarizer};

/// Create an LLM client based on provider/model flags or environment variables
fn create_llm_client(
    provider: Option<String>,
    model: Option<String>,
) -> Result<Arc<dyn retrochat_core::services::llm::LlmClient>> {
    // Determine provider from flag, env var, or default
    let llm_provider: LlmProvider = if let Some(p) = provider.as_deref() {
        p.parse::<LlmProvider>()
            .map_err(|e| anyhow::anyhow!("{e}"))?
    } else if let Ok(p) = std::env::var(env_llm::RETROCHAT_LLM_PROVIDER) {
        p.parse::<LlmProvider>()
            .map_err(|e| anyhow::anyhow!("{e}"))?
    } else {
        LlmProvider::GoogleAi
    };

    // Determine model from flag or env var
    let model_name = model.or_else(|| std::env::var(env_llm::RETROCHAT_LLM_MODEL).ok());

    // Build config based on provider
    let mut config = match llm_provider {
        LlmProvider::GoogleAi => {
            let api_key = std::env::var(env_apis::GOOGLE_AI_API_KEY).context(
                "GOOGLE_AI_API_KEY environment variable is required for google-ai provider",
            )?;
            LlmConfig::google_ai(api_key)
        }
        LlmProvider::ClaudeCode => {
            let mut cfg = LlmConfig::claude_code();
            if let Ok(path) = std::env::var(env_llm::CLAUDE_CODE_PATH) {
                cfg = cfg.with_cli_path(path);
            }
            cfg
        }
        LlmProvider::GeminiCli => {
            let mut cfg = LlmConfig::gemini_cli();
            if let Ok(path) = std::env::var(env_llm::GEMINI_CLI_PATH) {
                cfg = cfg.with_cli_path(path);
            }
            cfg
        }
    };

    if let Some(m) = model_name {
        config = config.with_model(m);
    }

    let client = LlmClientFactory::create(config)?;

    println!(
        "Using LLM provider: {} (model: {})",
        client.provider_name(),
        client.model_name()
    );

    Ok(client)
}

/// Handle the summarize turns command
pub async fn handle_summarize_turns(
    session_id: Option<String>,
    all: bool,
    provider: Option<String>,
    model: Option<String>,
) -> Result<()> {
    let db_path = retrochat_core::database::config::get_default_db_path()?;
    let db_manager = Arc::new(DatabaseManager::new(&db_path).await?);

    let llm_client = create_llm_client(provider, model)?;
    let summarizer = TurnSummarizer::new(&db_manager, llm_client);

    if all {
        summarize_all_sessions_turns(&db_manager, &summarizer).await
    } else if let Some(session_id) = session_id {
        let uuid = Uuid::parse_str(&session_id).context("Invalid session ID format")?;
        summarize_session_turns(&summarizer, &uuid).await
    } else {
        anyhow::bail!("Either provide a session ID or use --all flag")
    }
}

async fn summarize_session_turns(summarizer: &TurnSummarizer, session_id: &Uuid) -> Result<()> {
    println!("Summarizing turns for session {}...", session_id);

    let count = summarizer.summarize_session(session_id).await?;

    println!("Successfully summarized {} turns", count);
    Ok(())
}

async fn summarize_all_sessions_turns(
    db_manager: &DatabaseManager,
    summarizer: &TurnSummarizer,
) -> Result<()> {
    use retrochat_core::database::ChatSessionRepository;

    let session_repo = ChatSessionRepository::new(db_manager);
    let sessions = session_repo.get_all().await?;

    if sessions.is_empty() {
        println!("No sessions found to summarize");
        return Ok(());
    }

    println!("Found {} sessions to summarize", sessions.len());

    let mut success_count = 0;
    let mut error_count = 0;

    for session in &sessions {
        print!("Summarizing session {}... ", session.id);

        match summarizer.summarize_session(&session.id).await {
            Ok(count) => {
                println!("OK ({} turns)", count);
                success_count += 1;
            }
            Err(e) => {
                println!("FAILED: {}", e);
                error_count += 1;
            }
        }
    }

    println!(
        "\nCompleted: {} success, {} errors",
        success_count, error_count
    );
    Ok(())
}

/// Handle the summarize sessions command
pub async fn handle_summarize_sessions(
    session_id: Option<String>,
    all: bool,
    provider: Option<String>,
    model: Option<String>,
) -> Result<()> {
    let db_path = retrochat_core::database::config::get_default_db_path()?;
    let db_manager = Arc::new(DatabaseManager::new(&db_path).await?);

    let llm_client = create_llm_client(provider, model)?;
    let summarizer = SessionSummarizer::new(&db_manager, llm_client);

    if all {
        summarize_all_sessions(&db_manager, &summarizer).await
    } else if let Some(session_id) = session_id {
        let uuid = Uuid::parse_str(&session_id).context("Invalid session ID format")?;
        summarize_single_session(&summarizer, &uuid).await
    } else {
        anyhow::bail!("Either provide a session ID or use --all flag")
    }
}

async fn summarize_single_session(summarizer: &SessionSummarizer, session_id: &Uuid) -> Result<()> {
    println!("Generating session summary for {}...", session_id);

    let summary = summarizer.summarize_session(session_id).await?;

    println!("\nSession Summary:");
    println!("  Title: {}", summary.title);
    println!("  Summary: {}", summary.summary);
    if let Some(goal) = &summary.primary_goal {
        println!("  Goal: {}", goal);
    }
    if let Some(outcome) = &summary.outcome {
        println!("  Outcome: {}", outcome);
    }
    if let Some(tech) = &summary.technologies_used {
        println!("  Technologies: {}", tech.join(", "));
    }

    Ok(())
}

async fn summarize_all_sessions(
    db_manager: &DatabaseManager,
    summarizer: &SessionSummarizer,
) -> Result<()> {
    use retrochat_core::database::{ChatSessionRepository, TurnSummaryRepository};

    let session_repo = ChatSessionRepository::new(db_manager);
    let turn_summary_repo = TurnSummaryRepository::new(db_manager);

    let sessions = session_repo.get_all().await?;

    if sessions.is_empty() {
        println!("No sessions found to summarize");
        return Ok(());
    }

    // Only summarize sessions that have turn summaries
    let mut sessions_with_turns = Vec::new();
    for session in &sessions {
        let turn_count = turn_summary_repo.count_by_session(&session.id).await?;
        if turn_count > 0 {
            sessions_with_turns.push(session);
        }
    }

    if sessions_with_turns.is_empty() {
        println!("No sessions with turn summaries found. Run 'summarize turns --all' first.");
        return Ok(());
    }

    println!(
        "Found {} sessions with turn summaries",
        sessions_with_turns.len()
    );

    let mut success_count = 0;
    let mut error_count = 0;

    for session in &sessions_with_turns {
        print!("Summarizing session {}... ", session.id);

        match summarizer.summarize_session(&session.id).await {
            Ok(summary) => {
                println!("OK: {}", summary.title);
                success_count += 1;
            }
            Err(e) => {
                println!("FAILED: {}", e);
                error_count += 1;
            }
        }
    }

    println!(
        "\nCompleted: {} success, {} errors",
        success_count, error_count
    );
    Ok(())
}

/// Handle the summarize status command
pub async fn handle_summarize_status() -> Result<()> {
    let db_path = retrochat_core::database::config::get_default_db_path()?;
    let db_manager = Arc::new(DatabaseManager::new(&db_path).await?);

    use retrochat_core::database::{
        ChatSessionRepository, SessionSummaryRepository, TurnSummaryRepository,
    };

    let session_repo = ChatSessionRepository::new(&db_manager);
    let turn_summary_repo = TurnSummaryRepository::new(&db_manager);
    let session_summary_repo = SessionSummaryRepository::new(&db_manager);

    let sessions = session_repo.get_all().await?;

    if sessions.is_empty() {
        println!("No sessions found");
        return Ok(());
    }

    // Detect turns for each session
    let turn_detector = TurnDetector::new(&db_manager);

    println!("Session Summarization Status:");
    println!("{:-<80}", "");
    println!(
        "{:<36} {:>8} {:>10} {:>10} {:>10}",
        "Session ID", "Messages", "Turns", "Turn Sums", "Sess Sum"
    );
    println!("{:-<80}", "");

    let mut total_sessions = 0;
    let mut sessions_with_turn_summaries = 0;
    let mut sessions_with_session_summary = 0;

    for session in &sessions {
        total_sessions += 1;

        let detected_turns = turn_detector.detect_turns(&session.id).await?;
        let turn_summary_count = turn_summary_repo.count_by_session(&session.id).await?;
        let has_session_summary = session_summary_repo.exists_for_session(&session.id).await?;

        if turn_summary_count > 0 {
            sessions_with_turn_summaries += 1;
        }
        if has_session_summary {
            sessions_with_session_summary += 1;
        }

        let session_summary_status = if has_session_summary { "Yes" } else { "No" };

        println!(
            "{:<36} {:>8} {:>10} {:>10} {:>10}",
            session.id,
            session.message_count,
            detected_turns.len(),
            turn_summary_count,
            session_summary_status
        );
    }

    println!("{:-<80}", "");
    println!("\nSummary:");
    println!("  Total sessions: {}", total_sessions);
    println!(
        "  Sessions with turn summaries: {} ({:.1}%)",
        sessions_with_turn_summaries,
        (sessions_with_turn_summaries as f64 / total_sessions as f64) * 100.0
    );
    println!(
        "  Sessions with session summary: {} ({:.1}%)",
        sessions_with_session_summary,
        (sessions_with_session_summary as f64 / total_sessions as f64) * 100.0
    );

    Ok(())
}
