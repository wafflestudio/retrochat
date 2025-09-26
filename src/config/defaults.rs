use crate::models::prompt_template::{PromptTemplate, PromptVariable};

/// Default prompt templates for retrospection analysis
pub fn get_default_templates() -> Vec<PromptTemplate> {
    vec![
        create_session_summary_template(),
        create_improvement_analysis_template(),
        create_technical_review_template(),
        create_learning_insights_template(),
    ]
}

fn create_session_summary_template() -> PromptTemplate {
    let template_content = r#"Please analyze the following chat session and provide a comprehensive summary.

# Chat Session Content:
{chat_content}

# Analysis Request:
Provide a detailed retrospective analysis covering:

1. **Session Overview**
   - Main topics discussed
   - Key questions asked
   - Overall conversation flow

2. **Key Insights**
   - Important solutions or answers discovered
   - Notable patterns or recurring themes
   - Technical concepts explored

3. **Productivity Assessment**
   - How effective was this session?
   - Were goals achieved?
   - Areas where the conversation could have been more focused

4. **Action Items & Next Steps**
   - Follow-up tasks identified
   - Areas that need further exploration
   - Recommendations for future sessions

5. **Learning Summary**
   - New knowledge gained
   - Skills or concepts reinforced
   - Resources or references mentioned

Please be specific and actionable in your analysis."#;

    PromptTemplate::new(
        "session_summary",
        "Session Summary",
        "Comprehensive retrospective analysis of a chat session covering key insights, productivity, and learning outcomes",
        template_content,
        vec![
            PromptVariable::required("chat_content", "Complete chat session content including all messages and responses")
        ],
        "analysis"
    )
}

fn create_improvement_analysis_template() -> PromptTemplate {
    let template_content = r#"Analyze the following chat session to identify areas for improvement and optimization.

# Chat Session Content:
{chat_content}

# Improvement Analysis:

1. **Question Quality Assessment**
   - How clear and specific were the questions asked?
   - What questions could have been formulated better?
   - Were there missed opportunities for deeper inquiry?

2. **Information Gathering Efficiency**
   - Was the information collected systematically?
   - Were there redundant or unnecessary exchanges?
   - What information was missing that could have been helpful?

3. **Problem-Solving Approach**
   - How structured was the approach to solving problems?
   - Were alternative solutions explored?
   - Could the debugging/analysis process have been more efficient?

4. **Communication Patterns**
   - Areas where communication could be clearer
   - Times when more context should have been provided
   - Opportunities for better collaboration

5. **Concrete Improvement Suggestions**
   - Specific changes to make in future sessions
   - Tools or techniques that could help
   - Preparation steps for similar problems

Focus on actionable improvements that can be implemented immediately."#;

    PromptTemplate::new(
        "improvement_analysis",
        "Improvement Analysis",
        "Identifies specific areas for improvement in chat sessions and provides actionable recommendations",
        template_content,
        vec![
            PromptVariable::required("chat_content", "Complete chat session content for improvement analysis")
        ],
        "analysis"
    )
}

fn create_technical_review_template() -> PromptTemplate {
    let template_content = r#"Provide a technical review of the following chat session, focusing on code, architecture, and technical decisions.

# Chat Session Content:
{chat_content}

# Technical Review:

1. **Code Quality Assessment**
   - Code snippets reviewed and their quality
   - Best practices followed or missed
   - Security considerations addressed

2. **Architecture & Design Decisions**
   - System design choices discussed
   - Trade-offs considered
   - Scalability and maintainability factors

3. **Technology Choices**
   - Frameworks, libraries, or tools mentioned
   - Appropriateness of technical stack decisions
   - Alternative approaches that could be considered

4. **Problem-Solving Methodology**
   - Technical debugging approaches used
   - Systematic vs ad-hoc problem solving
   - Testing and validation strategies

5. **Documentation & Knowledge Transfer**
   - How well were technical concepts explained?
   - Areas where documentation could be improved
   - Knowledge gaps identified

6. **Technical Recommendations**
   - Specific technical improvements to implement
   - Resources for further technical learning
   - Next steps for technical development

Provide specific, actionable technical insights."#;

    PromptTemplate::new(
        "technical_review",
        "Technical Review",
        "In-depth technical analysis focusing on code quality, architecture decisions, and engineering practices",
        template_content,
        vec![
            PromptVariable::required("chat_content", "Chat session content with technical discussions and code")
        ],
        "technical"
    )
}

fn create_learning_insights_template() -> PromptTemplate {
    let template_content = r#"Extract learning insights and knowledge patterns from the following chat session.

# Chat Session Content:
{chat_content}

# Learning Insights Analysis:

1. **Knowledge Areas Explored**
   - Main subjects and topics covered
   - Depth of exploration in each area
   - Connections between different concepts

2. **Learning Progression**
   - How understanding evolved during the session
   - Breakthrough moments or key realizations
   - Areas where concepts clicked into place

3. **Skill Development**
   - Practical skills demonstrated or learned
   - Problem-solving techniques acquired
   - Tools or methodologies discovered

4. **Knowledge Gaps Identified**
   - Areas where further learning is needed
   - Concepts that need reinforcement
   - Prerequisites that should be studied

5. **Learning Resources & References**
   - Documentation, tutorials, or resources mentioned
   - Recommended reading or study materials
   - Communities or experts to follow up with

6. **Applied Learning Opportunities**
   - Practical projects to reinforce learning
   - Real-world applications of new knowledge
   - Ways to practice and solidify understanding

7. **Future Learning Path**
   - Logical next steps in the learning journey
   - Advanced topics to explore later
   - Related areas that would complement current knowledge

Focus on extracting maximum educational value from the session."#;

    PromptTemplate::new(
        "learning_insights",
        "Learning Insights",
        "Extracts educational value and knowledge patterns from chat sessions to optimize learning outcomes",
        template_content,
        vec![
            PromptVariable::required("chat_content", "Chat session content to analyze for learning patterns")
        ],
        "learning"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_templates_creation() {
        let templates = get_default_templates();
        assert_eq!(templates.len(), 4);

        let template_ids: Vec<&str> = templates.iter().map(|t| t.id.as_str()).collect();
        assert!(template_ids.contains(&"session_summary"));
        assert!(template_ids.contains(&"improvement_analysis"));
        assert!(template_ids.contains(&"technical_review"));
        assert!(template_ids.contains(&"learning_insights"));
    }

    #[test]
    fn test_session_summary_template() {
        let template = create_session_summary_template();
        assert_eq!(template.id, "session_summary");
        assert_eq!(template.category, "analysis");
        assert!(!template.template.is_empty());
        assert_eq!(template.variables.len(), 1);
        assert_eq!(template.variables[0].name, "chat_content");
        assert!(template.variables[0].required);
    }

    #[test]
    fn test_all_templates_have_required_fields() {
        let templates = get_default_templates();

        for template in &templates {
            assert!(!template.id.is_empty());
            assert!(!template.name.is_empty());
            assert!(!template.description.is_empty());
            assert!(!template.template.is_empty());
            assert!(!template.category.is_empty());
            assert!(!template.variables.is_empty());

            // All default templates should have chat_content variable
            assert!(template.variables.iter().any(|v| v.name == "chat_content"));
        }
    }

    #[test]
    fn test_template_content_contains_placeholder() {
        let templates = get_default_templates();

        for template in &templates {
            assert!(template.template.contains("{chat_content}"));
        }
    }
}
