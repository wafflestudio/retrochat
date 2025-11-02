use crate::services::ComprehensiveAnalysis;
use anyhow::Result;
use console::style;
use std::str::FromStr;

/// Output format for analytics reports
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Plain text format (current behavior)
    Plain,
    /// Enhanced format with markdown rendering and panels
    Enhanced,
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "plain" => Ok(Self::Plain),
            "enhanced" | "markdown" => Ok(Self::Enhanced),
            _ => Ok(Self::Enhanced), // Default to enhanced
        }
    }
}

impl OutputFormat {
    /// Parse format string with fallback to enhanced
    pub fn parse(s: &str) -> Self {
        s.parse().unwrap_or(Self::Enhanced)
    }
}

/// Terminal formatter for analytics reports
pub struct AnalyticsFormatter {
    format: OutputFormat,
    terminal_width: usize,
}

impl AnalyticsFormatter {
    pub fn new(format: OutputFormat) -> Self {
        let terminal_width = terminal_size::terminal_size()
            .map(|(w, _)| w.0 as usize)
            .unwrap_or(80);

        Self {
            format,
            terminal_width,
        }
    }

    /// Print the complete analysis report
    pub fn print_analysis(&self, analysis: &ComprehensiveAnalysis) -> Result<()> {
        match self.format {
            OutputFormat::Plain => self.print_plain(analysis),
            OutputFormat::Enhanced => self.print_enhanced(analysis),
        }
    }

    /// Print plain text format (preserves existing behavior)
    fn print_plain(&self, analysis: &ComprehensiveAnalysis) -> Result<()> {
        // This maintains backward compatibility with the existing output
        println!("\n🔍 Session Analysis Report");
        println!("==========================");
        println!("Session ID: {}", analysis.session_id);
        println!(
            "Generated: {}",
            analysis.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
        );

        self.print_quantitative_scores_plain(analysis);
        self.print_quantitative_metrics_plain(analysis);
        self.print_processed_statistics_plain(analysis);
        self.print_qualitative_insights_plain(analysis);

        println!("\n{}", "=".repeat(50));
        println!("Analysis complete! Use 'retrochat query sessions' to see other sessions.");
        println!("{}", "=".repeat(50));

        Ok(())
    }

    /// Print enhanced format with markdown and panels
    fn print_enhanced(&self, analysis: &ComprehensiveAnalysis) -> Result<()> {
        self.print_header(analysis)?;
        self.print_quantitative_scores_enhanced(analysis)?;
        self.print_quantitative_metrics_enhanced(analysis)?;
        self.print_processed_statistics_enhanced(analysis)?;
        self.print_qualitative_insights_enhanced(analysis)?;
        self.print_footer()?;

        Ok(())
    }

    // =============================================================================
    // Header and Footer
    // =============================================================================

    fn print_header(&self, analysis: &ComprehensiveAnalysis) -> Result<()> {
        let width = self.terminal_width;

        println!();
        println!("{}", style(self.box_top(width)).cyan());
        println!(
            "{} {} {}",
            style("│").cyan(),
            style("Session Analysis Report").bold().bright(),
            style("│").cyan()
        );
        println!("{}", style(self.box_separator(width)).cyan());
        println!(
            "{} Session ID: {}",
            style("│").cyan(),
            style(&analysis.session_id).yellow()
        );
        println!(
            "{} Generated: {}",
            style("│").cyan(),
            style(analysis.generated_at.format("%Y-%m-%d %H:%M:%S UTC")).dim()
        );
        println!("{}", style(self.box_bottom(width)).cyan());

        Ok(())
    }

    fn print_footer(&self) -> Result<()> {
        let width = self.terminal_width;

        println!();
        println!("{}", style(self.box_top(width)).green());
        println!(
            "{} {}",
            style("│").green(),
            style("Analysis complete!").bold()
        );
        println!(
            "{} Use 'retrochat query sessions' to see other sessions.",
            style("│").green()
        );
        println!("{}", style(self.box_bottom(width)).green());

        Ok(())
    }

    // =============================================================================
    // Quantitative Scores
    // =============================================================================

    fn print_quantitative_scores_plain(&self, analysis: &ComprehensiveAnalysis) {
        println!("\n📊 QUANTITATIVE SCORES");
        println!("─────────────────────");
        println!(
            "  Overall Score: {:.1}/100",
            analysis.quantitative_output.overall_score
        );
        println!(
            "  Code Quality: {:.1}/100",
            analysis.quantitative_output.code_quality_score
        );
        println!(
            "  Productivity: {:.1}/100",
            analysis.quantitative_output.productivity_score
        );
        println!(
            "  Efficiency: {:.1}/100",
            analysis.quantitative_output.efficiency_score
        );
        println!(
            "  Collaboration: {:.1}/100",
            analysis.quantitative_output.collaboration_score
        );
        println!(
            "  Learning: {:.1}/100",
            analysis.quantitative_output.learning_score
        );
    }

    fn print_quantitative_scores_enhanced(&self, analysis: &ComprehensiveAnalysis) -> Result<()> {
        let width = self.terminal_width;

        println!();
        println!("{}", style(self.box_top(width)).blue());
        println!(
            "{} {} {}",
            style("│").blue(),
            style("📊 QUANTITATIVE SCORES").bold().bright(),
            style("│").blue()
        );
        println!("{}", style(self.box_separator(width)).blue());

        // Print scores with visual bars
        self.print_score_bar("Overall", analysis.quantitative_output.overall_score);
        self.print_score_bar(
            "Code Quality",
            analysis.quantitative_output.code_quality_score,
        );
        self.print_score_bar(
            "Productivity",
            analysis.quantitative_output.productivity_score,
        );
        self.print_score_bar("Efficiency", analysis.quantitative_output.efficiency_score);
        self.print_score_bar(
            "Collaboration",
            analysis.quantitative_output.collaboration_score,
        );
        self.print_score_bar("Learning", analysis.quantitative_output.learning_score);

        println!("{}", style(self.box_bottom(width)).blue());

        Ok(())
    }

    fn print_score_bar(&self, label: &str, score: f64) {
        let bar_width = 30;
        let filled = ((score / 100.0) * bar_width as f64) as usize;
        let empty = bar_width - filled;

        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty));

        let colored_bar = if score >= 80.0 {
            style(bar).green()
        } else if score >= 60.0 {
            style(bar).yellow()
        } else {
            style(bar).red()
        };

        println!(
            "{} {:<15} {} {:.1}",
            style("│").blue(),
            label,
            colored_bar,
            score
        );
    }

    // =============================================================================
    // Quantitative Metrics
    // =============================================================================

    fn print_quantitative_metrics_plain(&self, analysis: &ComprehensiveAnalysis) {
        println!("\n📈 QUANTITATIVE METRICS");
        println!("─────────────────────");

        println!("\n📁 File Changes:");
        println!(
            "  Files Modified: {}",
            analysis
                .quantitative_input
                .file_changes
                .total_files_modified
        );
        println!(
            "  Files Read: {}",
            analysis.quantitative_input.file_changes.total_files_read
        );
        println!(
            "  Lines Added: {}",
            analysis.quantitative_input.file_changes.lines_added
        );
        println!(
            "  Lines Removed: {}",
            analysis.quantitative_input.file_changes.lines_removed
        );
        println!(
            "  Net Code Growth: {}",
            analysis.quantitative_input.file_changes.net_code_growth
        );
        println!(
            "  Refactoring Operations: {}",
            analysis
                .quantitative_input
                .file_changes
                .refactoring_operations
        );

        println!("\n⏱️ Time Metrics:");
        println!(
            "  Session Duration: {:.1} minutes",
            analysis
                .quantitative_input
                .time_metrics
                .total_session_time_minutes
        );
        println!(
            "  Peak Hours: {:?}",
            analysis.quantitative_input.time_metrics.peak_hours
        );

        println!("\n🔤 Token Metrics:");
        println!(
            "  Total Tokens: {}",
            analysis.quantitative_input.token_metrics.total_tokens_used
        );
        println!(
            "  Input Tokens: {}",
            analysis.quantitative_input.token_metrics.input_tokens
        );
        println!(
            "  Output Tokens: {}",
            analysis.quantitative_input.token_metrics.output_tokens
        );
        println!(
            "  Token Efficiency: {:.2}",
            analysis.quantitative_input.token_metrics.token_efficiency
        );

        println!("\n🛠️ Tool Usage:");
        println!(
            "  Total Operations: {}",
            analysis.quantitative_input.tool_usage.total_operations
        );
        println!(
            "  Successful: {}",
            analysis.quantitative_input.tool_usage.successful_operations
        );
        println!(
            "  Failed: {}",
            analysis.quantitative_input.tool_usage.failed_operations
        );

        if !analysis
            .quantitative_input
            .tool_usage
            .tool_distribution
            .is_empty()
        {
            println!("  Tool Distribution:");
            for (tool, count) in &analysis.quantitative_input.tool_usage.tool_distribution {
                println!("    {tool}: {count}");
            }
        }
    }

    fn print_quantitative_metrics_enhanced(&self, analysis: &ComprehensiveAnalysis) -> Result<()> {
        let width = self.terminal_width;

        println!();
        println!("{}", style(self.box_top(width)).magenta());
        println!(
            "{} {} {}",
            style("│").magenta(),
            style("📈 QUANTITATIVE METRICS").bold().bright(),
            style("│").magenta()
        );
        println!("{}", style(self.box_separator(width)).magenta());

        // File Changes
        println!(
            "{} {}",
            style("│").magenta(),
            style("📁 File Changes:").bold()
        );
        self.print_metric(
            "Files Modified",
            &analysis
                .quantitative_input
                .file_changes
                .total_files_modified
                .to_string(),
        );
        self.print_metric(
            "Files Read",
            &analysis
                .quantitative_input
                .file_changes
                .total_files_read
                .to_string(),
        );
        self.print_metric(
            "Lines Added",
            &format!("+{}", analysis.quantitative_input.file_changes.lines_added),
        );
        self.print_metric(
            "Lines Removed",
            &format!(
                "-{}",
                analysis.quantitative_input.file_changes.lines_removed
            ),
        );
        self.print_metric(
            "Net Code Growth",
            &analysis
                .quantitative_input
                .file_changes
                .net_code_growth
                .to_string(),
        );
        self.print_metric(
            "Refactoring Ops",
            &analysis
                .quantitative_input
                .file_changes
                .refactoring_operations
                .to_string(),
        );

        println!("{}", style(self.box_separator(width)).magenta());

        // Time Metrics
        println!(
            "{} {}",
            style("│").magenta(),
            style("⏱️  Time Metrics:").bold()
        );
        self.print_metric(
            "Session Duration",
            &format!(
                "{:.1} minutes",
                analysis
                    .quantitative_input
                    .time_metrics
                    .total_session_time_minutes
            ),
        );
        self.print_metric(
            "Peak Hours",
            &format!("{:?}", analysis.quantitative_input.time_metrics.peak_hours),
        );

        println!("{}", style(self.box_separator(width)).magenta());

        // Token Metrics
        println!(
            "{} {}",
            style("│").magenta(),
            style("🔤 Token Metrics:").bold()
        );
        self.print_metric(
            "Total Tokens",
            &analysis
                .quantitative_input
                .token_metrics
                .total_tokens_used
                .to_string(),
        );
        self.print_metric(
            "Input Tokens",
            &analysis
                .quantitative_input
                .token_metrics
                .input_tokens
                .to_string(),
        );
        self.print_metric(
            "Output Tokens",
            &analysis
                .quantitative_input
                .token_metrics
                .output_tokens
                .to_string(),
        );
        self.print_metric(
            "Token Efficiency",
            &format!(
                "{:.2}",
                analysis.quantitative_input.token_metrics.token_efficiency
            ),
        );

        println!("{}", style(self.box_separator(width)).magenta());

        // Tool Usage
        println!(
            "{} {}",
            style("│").magenta(),
            style("🛠️  Tool Usage:").bold()
        );
        self.print_metric(
            "Total Operations",
            &analysis
                .quantitative_input
                .tool_usage
                .total_operations
                .to_string(),
        );
        self.print_metric(
            "Successful",
            &format!(
                "{} ({:.1}%)",
                analysis.quantitative_input.tool_usage.successful_operations,
                (analysis.quantitative_input.tool_usage.successful_operations as f64
                    / analysis.quantitative_input.tool_usage.total_operations as f64)
                    * 100.0
            ),
        );
        self.print_metric(
            "Failed",
            &analysis
                .quantitative_input
                .tool_usage
                .failed_operations
                .to_string(),
        );

        if !analysis
            .quantitative_input
            .tool_usage
            .tool_distribution
            .is_empty()
        {
            println!(
                "{} {}",
                style("│").magenta(),
                style("  Tool Distribution:").dim()
            );
            let mut tools: Vec<_> = analysis
                .quantitative_input
                .tool_usage
                .tool_distribution
                .iter()
                .collect();
            tools.sort_by(|a, b| b.1.cmp(a.1));
            for (tool, count) in tools.iter().take(10) {
                println!(
                    "{}   {} {}",
                    style("│").magenta(),
                    style(format!("• {tool}")).cyan(),
                    style(format!("({count})")).dim()
                );
            }
        }

        println!("{}", style(self.box_bottom(width)).magenta());

        Ok(())
    }

    fn print_metric(&self, label: &str, value: &str) {
        println!(
            "{}   {:<20} {}",
            style("│").magenta(),
            style(label).dim(),
            style(value).bright()
        );
    }

    // =============================================================================
    // Processed Statistics
    // =============================================================================

    fn print_processed_statistics_plain(&self, analysis: &ComprehensiveAnalysis) {
        println!("\n⚙️ PROCESSED STATISTICS");
        println!("─────────────────────");

        println!("\n📊 Session Metrics:");
        println!(
            "  Total Sessions: {}",
            analysis.processed_output.session_metrics.total_sessions
        );
        println!(
            "  Avg Duration: {:.1} min",
            analysis
                .processed_output
                .session_metrics
                .average_session_duration_minutes
        );
        println!(
            "  Consistency Score: {:.1}%",
            analysis
                .processed_output
                .session_metrics
                .session_consistency_score
                * 100.0
        );

        println!("\n🔤 Token Statistics:");
        println!(
            "  Total Tokens: {}",
            analysis.processed_output.token_metrics.total_tokens
        );
        println!(
            "  Tokens/Hour: {:.1}",
            analysis.processed_output.token_metrics.tokens_per_hour
        );
        println!(
            "  I/O Ratio: {:.2}",
            analysis.processed_output.token_metrics.input_output_ratio
        );
        println!(
            "  Efficiency Score: {:.1}%",
            analysis
                .processed_output
                .token_metrics
                .token_efficiency_score
                * 100.0
        );
        println!(
            "  Cost Estimate: ${:.4}",
            analysis.processed_output.token_metrics.cost_estimate
        );

        println!("\n💻 Code Statistics:");
        println!(
            "  Net Lines Changed: {}",
            analysis
                .processed_output
                .code_change_metrics
                .net_lines_changed
        );
        println!(
            "  Files/Session: {:.1}",
            analysis
                .processed_output
                .code_change_metrics
                .files_per_session
        );
        println!(
            "  Lines/Hour: {:.1}",
            analysis.processed_output.code_change_metrics.lines_per_hour
        );
        println!(
            "  Refactoring Ratio: {:.1}%",
            analysis
                .processed_output
                .code_change_metrics
                .refactoring_ratio
                * 100.0
        );
        println!(
            "  Code Velocity: {:.1}",
            analysis.processed_output.code_change_metrics.code_velocity
        );

        println!("\n⏱️ Time Efficiency:");
        println!(
            "  Productivity Score: {:.1}%",
            analysis
                .processed_output
                .time_efficiency_metrics
                .productivity_score
                * 100.0
        );
        println!(
            "  Context Switching Cost: {:.1}%",
            analysis
                .processed_output
                .time_efficiency_metrics
                .context_switching_cost
                * 100.0
        );
        println!(
            "  Deep Work Ratio: {:.1}%",
            analysis
                .processed_output
                .time_efficiency_metrics
                .deep_work_ratio
                * 100.0
        );
        println!(
            "  Time Utilization: {:.1}%",
            analysis
                .processed_output
                .time_efficiency_metrics
                .time_utilization
                * 100.0
        );
    }

    fn print_processed_statistics_enhanced(&self, analysis: &ComprehensiveAnalysis) -> Result<()> {
        let width = self.terminal_width;

        println!();
        println!("{}", style(self.box_top(width)).yellow());
        println!(
            "{} {} {}",
            style("│").yellow(),
            style("⚙️  PROCESSED STATISTICS").bold().bright(),
            style("│").yellow()
        );
        println!("{}", style(self.box_separator(width)).yellow());

        // Session Metrics
        println!(
            "{} {}",
            style("│").yellow(),
            style("📊 Session Metrics:").bold()
        );
        self.print_stat(
            "Total Sessions",
            &analysis
                .processed_output
                .session_metrics
                .total_sessions
                .to_string(),
        );
        self.print_stat(
            "Avg Duration",
            &format!(
                "{:.1} min",
                analysis
                    .processed_output
                    .session_metrics
                    .average_session_duration_minutes
            ),
        );
        self.print_stat(
            "Consistency Score",
            &format!(
                "{:.1}%",
                analysis
                    .processed_output
                    .session_metrics
                    .session_consistency_score
                    * 100.0
            ),
        );

        println!("{}", style(self.box_separator(width)).yellow());

        // Token Statistics
        println!(
            "{} {}",
            style("│").yellow(),
            style("🔤 Token Statistics:").bold()
        );
        self.print_stat(
            "Total Tokens",
            &analysis
                .processed_output
                .token_metrics
                .total_tokens
                .to_string(),
        );
        self.print_stat(
            "Tokens/Hour",
            &format!(
                "{:.1}",
                analysis.processed_output.token_metrics.tokens_per_hour
            ),
        );
        self.print_stat(
            "I/O Ratio",
            &format!(
                "{:.2}",
                analysis.processed_output.token_metrics.input_output_ratio
            ),
        );
        self.print_stat(
            "Efficiency Score",
            &format!(
                "{:.1}%",
                analysis
                    .processed_output
                    .token_metrics
                    .token_efficiency_score
                    * 100.0
            ),
        );
        self.print_stat(
            "Cost Estimate",
            &format!(
                "${:.4}",
                analysis.processed_output.token_metrics.cost_estimate
            ),
        );

        println!("{}", style(self.box_separator(width)).yellow());

        // Code Statistics
        println!(
            "{} {}",
            style("│").yellow(),
            style("💻 Code Statistics:").bold()
        );
        self.print_stat(
            "Net Lines Changed",
            &analysis
                .processed_output
                .code_change_metrics
                .net_lines_changed
                .to_string(),
        );
        self.print_stat(
            "Files/Session",
            &format!(
                "{:.1}",
                analysis
                    .processed_output
                    .code_change_metrics
                    .files_per_session
            ),
        );
        self.print_stat(
            "Lines/Hour",
            &format!(
                "{:.1}",
                analysis.processed_output.code_change_metrics.lines_per_hour
            ),
        );
        self.print_stat(
            "Refactoring Ratio",
            &format!(
                "{:.1}%",
                analysis
                    .processed_output
                    .code_change_metrics
                    .refactoring_ratio
                    * 100.0
            ),
        );
        self.print_stat(
            "Code Velocity",
            &format!(
                "{:.1}",
                analysis.processed_output.code_change_metrics.code_velocity
            ),
        );

        println!("{}", style(self.box_separator(width)).yellow());

        // Time Efficiency
        println!(
            "{} {}",
            style("│").yellow(),
            style("⏱️  Time Efficiency:").bold()
        );
        self.print_stat(
            "Productivity Score",
            &format!(
                "{:.1}%",
                analysis
                    .processed_output
                    .time_efficiency_metrics
                    .productivity_score
                    * 100.0
            ),
        );
        self.print_stat(
            "Context Switching",
            &format!(
                "{:.1}%",
                analysis
                    .processed_output
                    .time_efficiency_metrics
                    .context_switching_cost
                    * 100.0
            ),
        );
        self.print_stat(
            "Deep Work Ratio",
            &format!(
                "{:.1}%",
                analysis
                    .processed_output
                    .time_efficiency_metrics
                    .deep_work_ratio
                    * 100.0
            ),
        );
        self.print_stat(
            "Time Utilization",
            &format!(
                "{:.1}%",
                analysis
                    .processed_output
                    .time_efficiency_metrics
                    .time_utilization
                    * 100.0
            ),
        );

        println!("{}", style(self.box_bottom(width)).yellow());

        Ok(())
    }

    fn print_stat(&self, label: &str, value: &str) {
        println!(
            "{}   {:<20} {}",
            style("│").yellow(),
            style(label).dim(),
            style(value).bright()
        );
    }

    // =============================================================================
    // Qualitative Insights
    // =============================================================================

    fn print_qualitative_insights_plain(&self, analysis: &ComprehensiveAnalysis) {
        println!("\n💭 QUALITATIVE INSIGHTS");
        println!("─────────────────────");

        println!("\n💡 Key Insights:");
        for insight in &analysis.qualitative_output.insights {
            println!(
                "  [{}] {}: {}",
                insight.category, insight.title, insight.description
            );
            println!("    Confidence: {:.1}%", insight.confidence * 100.0);
        }

        if !analysis.qualitative_output.good_patterns.is_empty() {
            println!("\n✅ Good Patterns:");
            for pattern in &analysis.qualitative_output.good_patterns {
                println!(
                    "  • {} ({} times): {}",
                    pattern.pattern_name, pattern.frequency, pattern.description
                );
                println!("    Impact: {}", pattern.impact);
            }
        }

        if !analysis.qualitative_output.improvement_areas.is_empty() {
            println!("\n🔧 Improvement Areas:");
            for area in &analysis.qualitative_output.improvement_areas {
                println!("  • {} [{}]", area.area_name, area.priority);
                println!("    Current: {}", area.current_state);
                println!("    Suggestion: {}", area.suggested_improvement);
                println!("    Expected Impact: {}", area.expected_impact);
            }
        }

        if !analysis.qualitative_output.recommendations.is_empty() {
            println!("\n🎯 Recommendations:");
            for rec in &analysis.qualitative_output.recommendations {
                println!("  • {}: {}", rec.title, rec.description);
                println!("    Impact Score: {:.1}/10", rec.impact_score * 10.0);
                println!("    Difficulty: {}", rec.implementation_difficulty);
            }
        }

        if !analysis.qualitative_output.learning_observations.is_empty() {
            println!("\n📚 Learning Observations:");
            for obs in &analysis.qualitative_output.learning_observations {
                println!("  • {} ({})", obs.observation, obs.skill_area);
                println!("    Progress: {}", obs.progress_indicator);
                if !obs.next_steps.is_empty() {
                    println!("    Next Steps: {}", obs.next_steps.join(", "));
                }
            }
        }
    }

    fn print_qualitative_insights_enhanced(&self, analysis: &ComprehensiveAnalysis) -> Result<()> {
        let width = self.terminal_width;

        println!();
        println!("{}", style(self.box_top(width)).green());
        println!(
            "{} {} {}",
            style("│").green(),
            style("💭 QUALITATIVE INSIGHTS").bold().bright(),
            style("│").green()
        );
        println!("{}", style(self.box_separator(width)).green());

        // Key Insights with markdown-style rendering
        if !analysis.qualitative_output.insights.is_empty() {
            println!("{} {}", style("│").green(), style("💡 Key Insights").bold());
            println!("{}", style("│").green());

            for insight in &analysis.qualitative_output.insights {
                let confidence_color = if insight.confidence >= 0.8 {
                    style(format!("{:.0}%", insight.confidence * 100.0)).green()
                } else if insight.confidence >= 0.6 {
                    style(format!("{:.0}%", insight.confidence * 100.0)).yellow()
                } else {
                    style(format!("{:.0}%", insight.confidence * 100.0)).red()
                };

                println!(
                    "{} {} {} {} {}",
                    style("│").green(),
                    style("  •").cyan(),
                    style(&insight.title).bold(),
                    style(format!("[{}]", insight.category)).dim(),
                    confidence_color
                );

                // Wrap description text
                for line in self.wrap_text(&insight.description, width - 8) {
                    println!("{}     {}", style("│").green(), style(line).dim());
                }
                println!("{}", style("│").green());
            }
        }

        // Good Patterns
        if !analysis.qualitative_output.good_patterns.is_empty() {
            println!("{}", style(self.box_separator(width)).green());
            println!(
                "{} {}",
                style("│").green(),
                style("✅ Good Patterns").bold()
            );
            println!("{}", style("│").green());

            for pattern in &analysis.qualitative_output.good_patterns {
                println!(
                    "{} {} {} {}",
                    style("│").green(),
                    style("  •").green(),
                    style(&pattern.pattern_name).bold(),
                    style(format!("({} times)", pattern.frequency)).dim()
                );

                for line in self.wrap_text(&pattern.description, width - 8) {
                    println!("{}     {}", style("│").green(), line);
                }

                println!(
                    "{}     {} {}",
                    style("│").green(),
                    style("Impact:").dim(),
                    style(&pattern.impact).cyan()
                );
                println!("{}", style("│").green());
            }
        }

        // Improvement Areas
        if !analysis.qualitative_output.improvement_areas.is_empty() {
            println!("{}", style(self.box_separator(width)).green());
            println!(
                "{} {}",
                style("│").green(),
                style("🔧 Improvement Areas").bold()
            );
            println!("{}", style("│").green());

            for area in &analysis.qualitative_output.improvement_areas {
                let priority_style = match area.priority.to_lowercase().as_str() {
                    "high" | "critical" => style(&area.priority).red(),
                    "medium" => style(&area.priority).yellow(),
                    _ => style(&area.priority).blue(),
                };

                println!(
                    "{} {} {} {}",
                    style("│").green(),
                    style("  •").yellow(),
                    style(&area.area_name).bold(),
                    priority_style
                );

                println!(
                    "{}     {} {}",
                    style("│").green(),
                    style("Current:").dim(),
                    &area.current_state
                );

                println!(
                    "{}     {} {}",
                    style("│").green(),
                    style("Suggestion:").dim(),
                    style(&area.suggested_improvement).cyan()
                );

                println!(
                    "{}     {} {}",
                    style("│").green(),
                    style("Impact:").dim(),
                    &area.expected_impact
                );
                println!("{}", style("│").green());
            }
        }

        // Recommendations
        if !analysis.qualitative_output.recommendations.is_empty() {
            println!("{}", style(self.box_separator(width)).green());
            println!(
                "{} {}",
                style("│").green(),
                style("🎯 Recommendations").bold()
            );
            println!("{}", style("│").green());

            for (idx, rec) in analysis
                .qualitative_output
                .recommendations
                .iter()
                .enumerate()
            {
                println!(
                    "{} {} {}",
                    style("│").green(),
                    style(format!("  {}.", idx + 1)).cyan(),
                    style(&rec.title).bold()
                );

                for line in self.wrap_text(&rec.description, width - 8) {
                    println!("{}     {}", style("│").green(), line);
                }

                println!(
                    "{}     {} {} | {} {}",
                    style("│").green(),
                    style("Impact:").dim(),
                    self.impact_visualization(rec.impact_score),
                    style("Difficulty:").dim(),
                    style(&rec.implementation_difficulty).yellow()
                );
                println!("{}", style("│").green());
            }
        }

        // Learning Observations
        if !analysis.qualitative_output.learning_observations.is_empty() {
            println!("{}", style(self.box_separator(width)).green());
            println!(
                "{} {}",
                style("│").green(),
                style("📚 Learning Observations").bold()
            );
            println!("{}", style("│").green());

            for obs in &analysis.qualitative_output.learning_observations {
                println!(
                    "{} {} {} {}",
                    style("│").green(),
                    style("  •").blue(),
                    style(&obs.observation).bold(),
                    style(format!("[{}]", obs.skill_area)).dim()
                );

                println!(
                    "{}     {} {}",
                    style("│").green(),
                    style("Progress:").dim(),
                    style(&obs.progress_indicator).cyan()
                );

                if !obs.next_steps.is_empty() {
                    println!("{}     {}", style("│").green(), style("Next Steps:").dim());
                    for step in &obs.next_steps {
                        println!(
                            "{}       {} {}",
                            style("│").green(),
                            style("→").cyan(),
                            step
                        );
                    }
                }
                println!("{}", style("│").green());
            }
        }

        println!("{}", style(self.box_bottom(width)).green());

        Ok(())
    }

    // =============================================================================
    // Helper Methods
    // =============================================================================

    fn box_top(&self, width: usize) -> String {
        format!("╭{}╮", "─".repeat(width.saturating_sub(2)))
    }

    fn box_bottom(&self, width: usize) -> String {
        format!("╰{}╯", "─".repeat(width.saturating_sub(2)))
    }

    fn box_separator(&self, width: usize) -> String {
        format!("├{}┤", "─".repeat(width.saturating_sub(2)))
    }

    fn wrap_text(&self, text: &str, max_width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        let mut current_line = String::new();

        for word in text.split_whitespace() {
            if current_line.len() + word.len() + 1 > max_width && !current_line.is_empty() {
                lines.push(current_line);
                current_line = String::new();
            }

            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        lines
    }

    fn impact_visualization(&self, impact: f64) -> console::StyledObject<String> {
        let score = (impact * 10.0) as usize;
        let filled = "●".repeat(score.min(10));
        let empty = "○".repeat((10 - score).max(0));

        if score >= 8 {
            style(format!("{filled}{empty}")).green()
        } else if score >= 5 {
            style(format!("{filled}{empty}")).yellow()
        } else {
            style(format!("{filled}{empty}")).red()
        }
    }
}
