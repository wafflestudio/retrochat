use retrochat::services::ParserService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Parser Service Demo ===\n");

    let parser_service = ParserService::new();

    // Parse the test file
    let sessions = parser_service
        .parse_file_from_path("/tmp/test_session.jsonl")
        .await?;

    println!("\n=== Summary ===");
    println!("Total sessions parsed: {}", sessions.len());

    Ok(())
}
