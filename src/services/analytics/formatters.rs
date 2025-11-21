use crate::models::Analytics;
use crate::services::analytics::models::{QualitativeCategoryList, QualitativeItem};
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
    pub fn print_analysis(&self, analysis: &Analytics) -> Result<()> {
        match self.format {
            OutputFormat::Plain => self.print_plain(analysis),
            OutputFormat::Enhanced => self.print_enhanced(analysis),
        }
    }

    /// Print plain text format (preserves existing behavior)
    fn print_plain(&self, analysis: &Analytics) -> Result<()> {
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
    fn print_enhanced(&self, analysis: &Analytics) -> Result<()> {
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

    fn print_header(&self, analysis: &Analytics) -> Result<()> {
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

    fn print_quantitative_scores_plain(&self, analysis: &Analytics) {
        println!("\n📊 QUANTITATIVE SCORES");
        println!("─────────────────────");
        println!("  Overall Score: {:.1}/100", analysis.scores.overall);
        println!("  Code Quality: {:.1}/100", analysis.scores.code_quality);
        println!("  Productivity: {:.1}/100", analysis.scores.productivity);
        println!("  Efficiency: {:.1}/100", analysis.scores.efficiency);
        println!("  Collaboration: {:.1}/100", analysis.scores.collaboration);
        println!("  Learning: {:.1}/100", analysis.scores.learning);
    }

    fn print_quantitative_scores_enhanced(&self, analysis: &Analytics) -> Result<()> {
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
        self.print_score_bar("Overall", analysis.scores.overall);
        self.print_score_bar("Code Quality", analysis.scores.code_quality);
        self.print_score_bar("Productivity", analysis.scores.productivity);
        self.print_score_bar("Efficiency", analysis.scores.efficiency);
        self.print_score_bar("Collaboration", analysis.scores.collaboration);
        self.print_score_bar("Learning", analysis.scores.learning);

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

    fn print_quantitative_metrics_plain(&self, analysis: &Analytics) {
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

    fn print_quantitative_metrics_enhanced(&self, analysis: &Analytics) -> Result<()> {
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

    fn print_processed_statistics_plain(&self, analysis: &Analytics) {
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

    fn print_processed_statistics_enhanced(&self, analysis: &Analytics) -> Result<()> {
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

    fn print_qualitative_insights_plain(&self, analysis: &Analytics) {
        println!("\n💭 QUALITATIVE INSIGHTS");
        println!("─────────────────────");

        let categories = QualitativeCategoryList::default_categories();

        for category in &categories.categories {
            let items = analysis.qualitative_output.items_by_category(&category.id);
            if items.is_empty() {
                continue;
            }

            let icon = self.get_category_icon_plain(&category.icon);
            println!("\n{} {}:", icon, category.name);

            for item in items {
                self.print_item_plain(item, category);
            }
        }
    }

    fn get_category_icon_plain(&self, icon: &str) -> &'static str {
        match icon {
            "lightbulb" => "💡",
            "check" => "✅",
            "wrench" => "🔧",
            "target" => "🎯",
            "book" => "📚",
            _ => "•",
        }
    }

    fn print_item_plain(
        &self,
        item: &QualitativeItem,
        category: &crate::services::analytics::models::QualitativeCategory,
    ) {
        // Print title with any relevant metadata inline
        let mut title_line = format!("  • {}", item.title);

        // Add category-specific inline metadata
        match category.id.as_str() {
            "insight" => {
                if let Some(cat) = item.get_string("category") {
                    title_line.push_str(&format!(" [{}]", cat));
                }
            }
            "good_pattern" => {
                if let Some(freq) = item.get_number("frequency") {
                    title_line.push_str(&format!(" ({:.0} times)", freq));
                }
            }
            "improvement" => {
                if let Some(priority) = item.get_string("priority") {
                    title_line.push_str(&format!(" [{}]", priority));
                }
            }
            _ => {}
        }

        println!("{}", title_line);
        println!("    {}", item.description);

        // Print category-specific metadata
        for field in &category.metadata_schema {
            if let Some(value) = item.metadata.get(&field.key) {
                // Skip fields already shown inline
                if matches!(
                    (category.id.as_str(), field.key.as_str()),
                    ("insight", "category")
                        | ("good_pattern", "frequency")
                        | ("improvement", "priority")
                ) {
                    continue;
                }

                let display_value = match value {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => {
                        if field.key.contains("score") || field.key.contains("confidence") {
                            format!("{:.1}%", n.as_f64().unwrap_or(0.0) * 100.0)
                        } else {
                            format!("{:.1}", n.as_f64().unwrap_or(0.0))
                        }
                    }
                    serde_json::Value::Array(arr) => arr
                        .iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                    _ => continue,
                };

                if !display_value.is_empty() {
                    println!("    {}: {}", field.display_name, display_value);
                }
            }
        }
    }

    fn print_qualitative_insights_enhanced(&self, analysis: &Analytics) -> Result<()> {
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

        let categories = QualitativeCategoryList::default_categories();
        let mut first_category = true;

        for category in &categories.categories {
            let items = analysis.qualitative_output.items_by_category(&category.id);
            if items.is_empty() {
                continue;
            }

            if !first_category {
                println!("{}", style(self.box_separator(width)).green());
            }
            first_category = false;

            let icon = self.get_category_icon_enhanced(&category.icon);
            println!(
                "{} {}",
                style("│").green(),
                style(format!("{} {}", icon, category.name)).bold()
            );
            println!("{}", style("│").green());

            for (idx, item) in items.iter().enumerate() {
                self.print_item_enhanced(item, category, idx, width);
            }
        }

        println!("{}", style(self.box_bottom(width)).green());

        Ok(())
    }

    fn get_category_icon_enhanced(&self, icon: &str) -> &'static str {
        match icon {
            "lightbulb" => "💡",
            "check" => "✅",
            "wrench" => "🔧",
            "target" => "🎯",
            "book" => "📚",
            _ => "•",
        }
    }

    fn print_item_enhanced(
        &self,
        item: &QualitativeItem,
        category: &crate::services::analytics::models::QualitativeCategory,
        idx: usize,
        width: usize,
    ) {
        // Build title line with inline metadata
        let bullet = match category.id.as_str() {
            "insight" => style("  •".to_string()).cyan(),
            "good_pattern" => style("  •".to_string()).green(),
            "improvement" => style("  •".to_string()).yellow(),
            "recommendation" => style(format!("  {}.", idx + 1)).cyan(),
            "learning" => style("  •".to_string()).blue(),
            _ => style("  •".to_string()).white(),
        };

        // Print title with inline metadata
        let mut inline_parts: Vec<String> = vec![];

        match category.id.as_str() {
            "insight" => {
                if let Some(cat) = item.get_string("category") {
                    inline_parts.push(format!("[{}]", cat));
                }
                if let Some(conf) = item.get_number("confidence") {
                    let conf_str = format!("{:.0}%", conf * 100.0);
                    let styled = if conf >= 0.8 {
                        style(conf_str).green().to_string()
                    } else if conf >= 0.6 {
                        style(conf_str).yellow().to_string()
                    } else {
                        style(conf_str).red().to_string()
                    };
                    inline_parts.push(styled);
                }
            }
            "good_pattern" => {
                if let Some(freq) = item.get_number("frequency") {
                    inline_parts.push(format!("({:.0} times)", freq));
                }
            }
            "improvement" => {
                if let Some(priority) = item.get_string("priority") {
                    let styled = match priority.to_lowercase().as_str() {
                        "high" | "critical" => style(priority).red().to_string(),
                        "medium" => style(priority).yellow().to_string(),
                        _ => style(priority).blue().to_string(),
                    };
                    inline_parts.push(styled);
                }
            }
            "learning" => {
                if let Some(skill) = item.get_string("skill_area") {
                    inline_parts.push(format!("[{}]", skill));
                }
            }
            _ => {}
        }

        let inline_suffix = if inline_parts.is_empty() {
            String::new()
        } else {
            format!(" {}", inline_parts.join(" "))
        };

        println!(
            "{} {} {}{}",
            style("│").green(),
            bullet,
            style(&item.title).bold(),
            style(&inline_suffix).dim()
        );

        // Wrap and print description
        for line in self.wrap_text(&item.description, width - 8) {
            println!("{}     {}", style("│").green(), style(line).dim());
        }

        // Print remaining metadata fields
        for field in &category.metadata_schema {
            // Skip fields already shown inline
            let skip_inline = matches!(
                (category.id.as_str(), field.key.as_str()),
                ("insight", "category")
                    | ("insight", "confidence")
                    | ("good_pattern", "frequency")
                    | ("improvement", "priority")
                    | ("learning", "skill_area")
            );

            if skip_inline {
                continue;
            }

            if let Some(value) = item.metadata.get(&field.key) {
                match value {
                    serde_json::Value::String(s) if !s.is_empty() => {
                        println!(
                            "{}     {} {}",
                            style("│").green(),
                            style(format!("{}:", field.display_name)).dim(),
                            style(s).cyan()
                        );
                    }
                    serde_json::Value::Number(n) => {
                        let display = if field.key.contains("score") {
                            self.impact_visualization(n.as_f64().unwrap_or(0.0))
                        } else {
                            style(format!("{:.1}", n.as_f64().unwrap_or(0.0))).cyan()
                        };
                        println!(
                            "{}     {} {}",
                            style("│").green(),
                            style(format!("{}:", field.display_name)).dim(),
                            display
                        );
                    }
                    serde_json::Value::Array(arr) if !arr.is_empty() => {
                        println!(
                            "{}     {}",
                            style("│").green(),
                            style(format!("{}:", field.display_name)).dim()
                        );
                        for v in arr {
                            if let Some(s) = v.as_str() {
                                println!(
                                    "{}       {} {}",
                                    style("│").green(),
                                    style("→").cyan(),
                                    s
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        println!("{}", style("│").green());
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
