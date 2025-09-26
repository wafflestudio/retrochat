// Integration test for custom prompt analysis scenario
// This test MUST FAIL until prompt template management is implemented

use anyhow::Result;
use chrono::Utc;
use retrochat::cli::analytics::AnalyticsCommand;
use retrochat::cli::prompts::PromptsCommand;
use retrochat::database::chat_session_repo::ChatSessionRepository;
use retrochat::database::connection::DatabaseManager;
use retrochat::database::message_repo::MessageRepository;
use retrochat::models::chat_session::ChatSession;
use retrochat::models::message::{Message, MessageRole};
use retrochat::models::prompt_template::{PromptTemplate, PromptVariable};
use retrochat::models::LlmProvider;
use std::env;
use uuid::Uuid;

/// Test the complete custom prompt analysis workflow from quickstart scenario 2
#[tokio::test]
async fn test_custom_prompt_analysis_workflow() -> Result<()> {
    // Skip test if no API key is available
    if env::var("GEMINI_API_KEY").is_err() {
        println!("Skipping custom prompt test - GEMINI_API_KEY not set");
        return Ok(());
    }

    let db_manager = DatabaseManager::new(":memory:")?;

    // Create test session for project management analysis
    let session_id = Uuid::new_v4();
    let session = ChatSession {
        id: session_id,
        provider: LlmProvider::ClaudeCode,
        project_name: Some("project_alpha".to_string()),
        start_time: Utc::now(),
        end_time: Some(Utc::now()),
        message_count: 3,
        token_count: Some(250),
        file_path: "/test/project_alpha".to_string(),
        file_hash: "alpha_hash".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        state: retrochat::models::chat_session::SessionState::Completed,
    };

    let session_repo = ChatSessionRepository::new(db_manager.clone());
    session_repo.create(&session)?;

    // Add project management focused conversation
    let message_repo = MessageRepository::new(db_manager.clone());
    let messages = vec![
        Message {
            id: Uuid::new_v4(),
            session_id,
            role: Role::User,
            content: "We're behind on the Q2 deliverables. The team is blocked on API integration and the design system isn't finalized.".to_string(),
            timestamp: Utc::now(),
            token_count: Some(80),
            tool_calls: None,
            metadata: None,
            sequence_number: 0,
        },
        Message {
            id: Uuid::new_v4(),
            session_id,
            role: Role::Assistant,
            content: "Let's prioritize these blockers. For the API integration, I suggest creating a mock service first. For the design system, can we identify the minimum viable components needed for the current sprint?".to_string(),
            timestamp: Utc::now(),
            token_count: Some(90),
            tool_calls: None,
            metadata: None,
            sequence_number: 1,
        },
        Message {
            id: Uuid::new_v4(),
            session_id,
            role: Role::User,
            content: "Good idea. The critical components are buttons, forms, and navigation. Should we schedule a team sync to align on priorities?".to_string(),
            timestamp: Utc::now(),
            token_count: Some(80),
            tool_calls: None,
            metadata: None,
            sequence_number: 2,
        },
    ];

    for message in messages {
        message_repo.create(&message)?;
    }

    // Test CLI: retrochat prompts list
    let prompts_cmd = PromptsCommand::new(db_manager.clone());
    let initial_templates = prompts_cmd.list_templates().await?;
    let initial_count = initial_templates.len();

    // Test CLI: retrochat prompts create "project_analysis"
    let custom_template = PromptTemplate {
        id: "project_analysis".to_string(),
        name: "Project Management Analysis".to_string(),
        description: "Analyze chat for project management insights".to_string(),
        template: "Focus on: project milestones, blockers, and team communication patterns.\n\nConversation to analyze:\n{chat_content}\n\nProvide insights on:\n1. **Project Milestones**: What deliverables or deadlines were mentioned?\n2. **Blockers**: What obstacles or challenges were identified?\n3. **Team Communication**: How effective was the problem-solving discussion?\n4. **Action Items**: What next steps were proposed or agreed upon?".to_string(),
        variables: vec![
            PromptVariable {
                name: "chat_content".to_string(),
                description: "The chat session content to analyze".to_string(),
                required: true,
                default_value: None,
            }
        ],
        category: "custom".to_string(),
        is_default: false,
        created_at: Utc::now(),
        modified_at: Utc::now(),
    };

    prompts_cmd.create_template(&custom_template).await?;

    // Verify template was created
    let updated_templates = prompts_cmd.list_templates().await?;
    assert_eq!(
        updated_templates.len(),
        initial_count + 1,
        "Should have one more template"
    );

    let created_template = updated_templates
        .iter()
        .find(|t| t.id == "project_analysis")
        .expect("Should find created template");
    assert_eq!(created_template.name, "Project Management Analysis");

    // Test CLI: retrochat analyze retrospect --session <session-id> --template "project_analysis"
    let analytics_cmd = AnalyticsCommand::new(db_manager.clone());
    let result = analytics_cmd
        .execute_retrospect_analysis(session_id, Some("project_analysis".to_string()))
        .await?;

    // Verify custom analysis was created
    assert!(result.is_some(), "Custom analysis should be created");
    let analysis = result.unwrap();

    assert_eq!(analysis.session_id, session_id);
    assert_eq!(analysis.prompt_template_id, "project_analysis");
    assert!(
        !analysis.analysis_content.is_empty(),
        "Analysis content should not be empty"
    );

    // Verify analysis focuses on project management aspects
    let content = analysis.analysis_content.to_lowercase();
    let project_keywords = [
        "milestone",
        "blocker",
        "deliverable",
        "priority",
        "team",
        "sprint",
    ];
    let has_project_focus = project_keywords
        .iter()
        .any(|&keyword| content.contains(keyword));

    assert!(
        has_project_focus,
        "Analysis should contain project management insights. Content: {}",
        analysis.analysis_content
    );

    // Test CLI: retrochat analyze show --session <session-id> --analysis <analysis-id>
    let specific_analysis = analytics_cmd.get_analysis_by_id(analysis.id).await?;
    assert!(
        specific_analysis.is_some(),
        "Should retrieve specific analysis"
    );
    assert_eq!(specific_analysis.unwrap().id, analysis.id);

    Ok(())
}

/// Test template validation and error handling
#[tokio::test]
async fn test_custom_template_validation() -> Result<()> {
    let db_manager = DatabaseManager::new(":memory:")?;
    let prompts_cmd = PromptsCommand::new(db_manager);

    // Test invalid template (missing required variable)
    let invalid_template = PromptTemplate {
        id: "invalid_template".to_string(),
        name: "Invalid Template".to_string(),
        description: "Template with missing variables".to_string(),
        template: "This template references {missing_var} but doesn't define it".to_string(),
        variables: vec![PromptVariable {
            name: "chat_content".to_string(),
            description: "Chat content".to_string(),
            required: true,
            default_value: None,
        }],
        category: "custom".to_string(),
        is_default: false,
        created_at: Utc::now(),
        modified_at: Utc::now(),
    };

    let result = prompts_cmd.create_template(&invalid_template).await;
    assert!(
        result.is_err(),
        "Should reject template with undefined variables"
    );

    // Test template with duplicate ID
    let template1 = PromptTemplate {
        id: "duplicate_id".to_string(),
        name: "First Template".to_string(),
        description: "First template".to_string(),
        template: "Content: {chat_content}".to_string(),
        variables: vec![PromptVariable {
            name: "chat_content".to_string(),
            description: "Chat content".to_string(),
            required: true,
            default_value: None,
        }],
        category: "custom".to_string(),
        is_default: false,
        created_at: Utc::now(),
        modified_at: Utc::now(),
    };

    prompts_cmd.create_template(&template1).await?;

    let template2 = PromptTemplate {
        id: "duplicate_id".to_string(),
        name: "Second Template".to_string(),
        description: "Duplicate ID template".to_string(),
        template: "Different content: {chat_content}".to_string(),
        variables: vec![PromptVariable {
            name: "chat_content".to_string(),
            description: "Chat content".to_string(),
            required: true,
            default_value: None,
        }],
        category: "custom".to_string(),
        is_default: false,
        created_at: Utc::now(),
        modified_at: Utc::now(),
    };

    let result = prompts_cmd.create_template(&template2).await;
    assert!(result.is_err(), "Should reject template with duplicate ID");

    Ok(())
}

/// Test template editing and deletion
#[tokio::test]
async fn test_template_lifecycle() -> Result<()> {
    let db_manager = DatabaseManager::new(":memory:")?;
    let prompts_cmd = PromptsCommand::new(db_manager);

    // Create template
    let template = PromptTemplate {
        id: "test_template".to_string(),
        name: "Test Template".to_string(),
        description: "Original description".to_string(),
        template: "Original template: {chat_content}".to_string(),
        variables: vec![PromptVariable {
            name: "chat_content".to_string(),
            description: "Chat content".to_string(),
            required: true,
            default_value: None,
        }],
        category: "custom".to_string(),
        is_default: false,
        created_at: Utc::now(),
        modified_at: Utc::now(),
    };

    prompts_cmd.create_template(&template).await?;

    // Edit template
    let updated_template = PromptTemplate {
        id: "test_template".to_string(),
        name: "Updated Test Template".to_string(),
        description: "Updated description".to_string(),
        template: "Updated template: {chat_content}".to_string(),
        variables: vec![PromptVariable {
            name: "chat_content".to_string(),
            description: "Updated chat content description".to_string(),
            required: true,
            default_value: None,
        }],
        category: "custom".to_string(),
        is_default: false,
        created_at: template.created_at,
        modified_at: Utc::now(),
    };

    prompts_cmd.update_template(&updated_template).await?;

    // Verify update
    let retrieved = prompts_cmd.get_template("test_template").await?;
    assert!(retrieved.is_some(), "Template should exist after update");
    let retrieved_template = retrieved.unwrap();
    assert_eq!(retrieved_template.name, "Updated Test Template");
    assert_eq!(retrieved_template.description, "Updated description");

    // Delete template
    prompts_cmd.delete_template("test_template").await?;

    // Verify deletion
    let deleted = prompts_cmd.get_template("test_template").await?;
    assert!(
        deleted.is_none(),
        "Template should not exist after deletion"
    );

    Ok(())
}
