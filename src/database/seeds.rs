use crate::models::{PromptTemplate, PromptVariable};
use anyhow::Result;
use rusqlite::Connection;

use super::prompt_template_repo::PromptTemplateRepository;

pub fn seed_default_prompt_templates(conn: &Connection) -> Result<()> {
    let repo = PromptTemplateRepository::new(conn);

    // Check if default templates already exist
    let existing_defaults = repo.list_default_templates()?;
    if !existing_defaults.is_empty() {
        tracing::info!("Default prompt templates already exist, skipping seeding");
        return Ok(());
    }

    tracing::info!("Seeding default prompt templates");

    // 1. Basic Session Analysis Template
    let basic_analysis_template = PromptTemplate::default_template(
        "basic-session-analysis",
        "Basic Session Analysis",
        "Analyzes chat session for general insights, patterns, and retrospective observations",
        r#"Please analyze the following chat session and provide insights:

**Chat Session Content:**
{chat_content}

**Analysis Instructions:**
Provide a comprehensive analysis of this chat session including:

1. **Session Overview**
   - Purpose and context of the conversation
   - Key topics discussed
   - Overall conversation flow

2. **Key Insights**
   - Main insights or learnings from the session
   - Problem-solving approaches used
   - Decision points and outcomes

3. **Patterns & Observations**
   - Communication patterns observed
   - Recurring themes or topics
   - Quality of interactions

4. **Retrospective Analysis**
   - What worked well in this session
   - Areas for improvement
   - Suggestions for future similar sessions

5. **Action Items**
   - Follow-up tasks identified
   - Knowledge gaps to address
   - Resources or tools mentioned

Please provide specific examples from the conversation to support your analysis. Focus on actionable insights that could be valuable for future reference."#,
        vec![PromptVariable::required(
            "chat_content",
            "The complete chat session content to analyze",
        )],
        "analysis",
    );

    repo.create(&basic_analysis_template)?;

    // 2. Code Review Analysis Template
    let code_review_template = PromptTemplate::default_template(
        "code-review-analysis",
        "Code Review & Development Analysis",
        "Focused analysis for sessions involving code review, debugging, and development discussions",
        r#"Analyze this development-focused chat session:

**Chat Session Content:**
{chat_content}

**Development Analysis:**

1. **Technical Overview**
   - Programming languages and technologies discussed
   - Code review points covered
   - Architecture or design decisions made

2. **Code Quality Insights**
   - Code quality issues identified
   - Best practices discussed
   - Performance considerations mentioned

3. **Problem Solving**
   - Bugs or issues encountered
   - Debugging approaches used
   - Solutions implemented or proposed

4. **Learning Points**
   - New techniques or patterns learned
   - Documentation or resources shared
   - Knowledge transfer moments

5. **Development Process**
   - Code review effectiveness
   - Collaboration patterns
   - Process improvements identified

6. **Action Items**
   - Code changes to implement
   - Documentation to update
   - Follow-up research needed

Focus on technical accuracy and provide specific examples from the development discussion."#,
        vec![
            PromptVariable::required("chat_content", "The complete chat session content to analyze"),
        ],
        "development",
    );

    repo.create(&code_review_template)?;

    // 3. Project Planning Analysis Template
    let project_planning_template = PromptTemplate::default_template(
        "project-planning-analysis",
        "Project Planning & Strategy Analysis",
        "Analyzes sessions focused on project planning, strategy discussions, and decision-making",
        r#"Analyze this project planning session:

**Chat Session Content:**
{chat_content}

**Project Analysis:**

1. **Project Context**
   - Project scope and objectives
   - Stakeholders and requirements discussed
   - Timeline and resource considerations

2. **Strategic Decisions**
   - Key decisions made during planning
   - Alternative approaches considered
   - Risk assessment and mitigation strategies

3. **Planning Quality**
   - Thoroughness of planning process
   - Clarity of requirements and scope
   - Feasibility assessment accuracy

4. **Resource Planning**
   - Team structure and roles defined
   - Technology stack decisions
   - Budget and timeline estimates

5. **Communication Effectiveness**
   - Clarity of goals and expectations
   - Stakeholder alignment achieved
   - Documentation and follow-up plans

6. **Next Steps**
   - Immediate action items
   - Milestone definitions
   - Success criteria established

Provide insights on the planning process effectiveness and strategic soundness of decisions made."#,
        vec![PromptVariable::required(
            "chat_content",
            "The complete chat session content to analyze",
        )],
        "planning",
    );

    repo.create(&project_planning_template)?;

    // 4. Learning & Research Analysis Template
    let learning_template = PromptTemplate::default_template(
        "learning-research-analysis",
        "Learning & Research Session Analysis",
        "Analyzes sessions focused on learning new concepts, research, and knowledge acquisition",
        r#"Analyze this learning-focused session:

**Chat Session Content:**
{chat_content}

**Learning Analysis:**

1. **Learning Objectives**
   - Topics and concepts explored
   - Learning goals identified
   - Knowledge gaps addressed

2. **Knowledge Acquisition**
   - New concepts successfully learned
   - Complexity of topics covered
   - Understanding progression observed

3. **Research Effectiveness**
   - Quality of explanations provided
   - Resources and references shared
   - Depth of topic exploration

4. **Learning Patterns**
   - Question formulation effectiveness
   - Follow-up inquiry quality
   - Comprehension verification methods

5. **Knowledge Retention**
   - Key takeaways identified
   - Practical applications discussed
   - Memory anchors created

6. **Further Learning**
   - Additional topics to explore
   - Recommended resources
   - Practice opportunities identified

Focus on the learning process effectiveness and knowledge acquisition quality."#,
        vec![PromptVariable::required(
            "chat_content",
            "The complete chat session content to analyze",
        )],
        "learning",
    );

    repo.create(&learning_template)?;

    // 5. Problem Solving Analysis Template
    let problem_solving_template = PromptTemplate::default_template(
        "problem-solving-analysis",
        "Problem Solving & Troubleshooting Analysis",
        "Analyzes sessions focused on problem-solving, debugging, and troubleshooting activities",
        r#"Analyze this problem-solving session:

**Chat Session Content:**
{chat_content}

**Problem Solving Analysis:**

1. **Problem Definition**
   - Clarity of problem statement
   - Scope and impact assessment
   - Constraints and requirements identified

2. **Solution Approach**
   - Problem-solving methodology used
   - Alternative solutions considered
   - Decision-making process quality

3. **Troubleshooting Process**
   - Diagnostic steps taken
   - Information gathering effectiveness
   - Root cause analysis depth

4. **Solution Quality**
   - Effectiveness of final solution
   - Sustainability and maintainability
   - Implementation considerations

5. **Process Efficiency**
   - Time to resolution
   - Resource utilization
   - Collaboration effectiveness

6. **Learning & Improvement**
   - Lessons learned from the process
   - Process improvements identified
   - Knowledge documented for future use

Evaluate the problem-solving effectiveness and provide insights for improvement."#,
        vec![PromptVariable::required(
            "chat_content",
            "The complete chat session content to analyze",
        )],
        "problem-solving",
    );

    repo.create(&problem_solving_template)?;

    // 6. Custom Analysis Template with Focus
    let custom_focused_template = PromptTemplate::default_template(
        "custom-focused-analysis",
        "Custom Focused Analysis",
        "Flexible template for analyzing sessions with a specific focus area or custom criteria",
        r#"Analyze this chat session with focus on: {analysis_focus}

**Chat Session Content:**
{chat_content}

**Focused Analysis Instructions:**

1. **Primary Focus: {analysis_focus}**
   - Analyze the session specifically through the lens of {analysis_focus}
   - Identify patterns, insights, and observations related to {analysis_focus}
   - Evaluate effectiveness and quality in the context of {analysis_focus}

2. **Detailed Analysis**
   - Key points relevant to {analysis_focus}
   - Strengths and areas for improvement
   - Specific examples and evidence from the conversation

3. **Insights & Patterns**
   - Recurring themes related to {analysis_focus}
   - Successful approaches and techniques
   - Challenges and obstacles encountered

4. **Quality Assessment**
   - Effectiveness in achieving goals related to {analysis_focus}
   - Communication quality and clarity
   - Outcome satisfaction and completeness

5. **Recommendations**
   - Specific improvements for {analysis_focus}
   - Best practices to apply in future sessions
   - Resources or tools that could help

6. **Action Items**
   - Follow-up tasks related to {analysis_focus}
   - Knowledge gaps to address
   - Next steps for continued improvement

{additional_instructions}

Provide specific, actionable insights focused on {analysis_focus} with concrete examples from the session."#,
        vec![
            PromptVariable::required("chat_content", "The complete chat session content to analyze"),
            PromptVariable::required("analysis_focus", "The specific focus area or topic for analysis (e.g., 'communication patterns', 'technical accuracy', 'decision-making process')"),
            PromptVariable::optional("additional_instructions", "Any additional specific instructions for the analysis", "Focus on practical insights and actionable recommendations."),
        ],
        "custom",
    );

    repo.create(&custom_focused_template)?;

    tracing::info!("Successfully seeded {} default prompt templates", 6);
    Ok(())
}

pub fn verify_default_templates(conn: &Connection) -> Result<bool> {
    let repo = PromptTemplateRepository::new(conn);

    let expected_templates = vec![
        "basic-session-analysis",
        "code-review-analysis",
        "project-planning-analysis",
        "learning-research-analysis",
        "problem-solving-analysis",
        "custom-focused-analysis",
    ];

    for template_id in &expected_templates {
        if repo.find_by_id(template_id)?.is_none() {
            tracing::warn!("Default template '{}' not found", template_id);
            return Ok(false);
        }
    }

    tracing::info!("All default prompt templates verified successfully");
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::schema::create_schema;
    use rusqlite::Connection;

    #[test]
    fn test_seed_default_templates() {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();

        // Test seeding
        assert!(seed_default_prompt_templates(&conn).is_ok());

        // Verify templates were created
        assert!(verify_default_templates(&conn).unwrap());

        // Test that seeding again doesn't create duplicates
        assert!(seed_default_prompt_templates(&conn).is_ok());

        let repo = PromptTemplateRepository::new(&conn);
        let default_templates = repo.list_default_templates().unwrap();
        assert_eq!(default_templates.len(), 6);
    }

    #[test]
    fn test_template_validation() {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();
        seed_default_prompt_templates(&conn).unwrap();

        let repo = PromptTemplateRepository::new(&conn);

        // Test each template validates correctly
        let templates = repo.list_default_templates().unwrap();
        for template in templates {
            assert!(
                template.validate().is_ok(),
                "Template '{}' failed validation: {:?}",
                template.id,
                template.validate()
            );
        }
    }

    #[test]
    fn test_template_rendering() {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();
        seed_default_prompt_templates(&conn).unwrap();

        let repo = PromptTemplateRepository::new(&conn);

        // Test basic template rendering
        let template = repo.find_by_id("basic-session-analysis").unwrap().unwrap();
        let mut variables = std::collections::HashMap::new();
        variables.insert("chat_content".to_string(), "Test chat content".to_string());

        let rendered = template.render(&variables).unwrap();
        assert!(rendered.contains("Test chat content"));
        assert!(!rendered.contains("{chat_content}"));
    }
}
