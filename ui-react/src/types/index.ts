export interface Session {
  id: string
  provider: string
  project_name: string | null
  message_count: number
  created_at: string
  updated_at: string
}

export interface FileMetadata {
  file_path: string
  file_extension: string | null
  is_code_file: boolean | null
  lines_added: number | null
  lines_removed: number | null
}

export interface ToolOperation {
  id: string
  tool_use_id: string
  tool_name: string
  timestamp: string
  success: boolean | null
  result_summary: string | null
  file_metadata: FileMetadata | null
  bash_metadata: Record<string, unknown> | null
}

export interface Message {
  id: string
  role: string
  content: string
  timestamp: string
  message_type: string
  tool_operation: ToolOperation | null
}

export interface SessionWithMessages extends Session {
  messages: Message[]
}

export interface SearchResult {
  session_id: string
  role: string
  provider: string
  content: string
  timestamp: string
}

// Analytics types
export type OperationStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled'

export interface AnalyticsRequest {
  id: string
  session_id: string
  status: OperationStatus
  started_at: string
  completed_at: string | null
  error_message: string | null
}

export interface FileChangeMetrics {
  total_files_modified: number
  total_files_read: number
  lines_added: number
  lines_removed: number
  net_code_growth: number
}

export interface TimeConsumptionMetrics {
  total_session_time_minutes: number
  peak_hours: number[]
}

export interface TokenConsumptionMetrics {
  total_tokens_used: number
  input_tokens: number
  output_tokens: number
  token_efficiency: number
}

export interface ToolUsageMetrics {
  total_operations: number
  successful_operations: number
  failed_operations: number
  tool_distribution: Record<string, number>
  average_execution_time_ms: number
}

export interface MetricQuantitativeOutput {
  file_changes: FileChangeMetrics
  time_metrics: TimeConsumptionMetrics
  token_metrics: TokenConsumptionMetrics
  tool_usage: ToolUsageMetrics
}

// Summary of qualitative evaluation
export interface QualitativeEvaluationSummary {
  total_entries: number
  categories_evaluated: number
  entries_version: string
}

// Output structure for a qualitative entry with analysis results
export interface QualitativeEntryOutput {
  // Unique key for the entry (e.g., "insights", "good_patterns")
  key: string
  // Display title (e.g., "Insights", "Good Patterns")
  title: string
  // Full description of what this entry measures
  description: string
  // Short one-line summary of the analysis results
  short_description: string
  // List of specific observations/items for this entry
  items: string[]
}

// AI-generated qualitative output from configurable entry-based analysis
// Each entry contains key, title, description, short_description, and items
export interface AIQualitativeOutput {
  // List of qualitative entry outputs with full metadata and analysis results
  entries: QualitativeEntryOutput[]
  // Summary of qualitative evaluation
  summary: QualitativeEvaluationSummary | null
  // Version of qualitative entries configuration used
  entries_version: string | null
}

// Rubric-based evaluation types (LLM-as-a-judge)
export interface RubricScore {
  rubric_id: string
  rubric_name: string
  score: number
  max_score: number
  reasoning: string
}

export interface RubricEvaluationSummary {
  total_score: number
  max_score: number
  percentage: number
  rubrics_evaluated: number
  rubrics_version: string
}

// AI-generated quantitative output from rubric-based LLM-as-a-judge evaluation
export interface AIQuantitativeOutput {
  rubric_scores: RubricScore[]
  rubric_summary: RubricEvaluationSummary | null
}

export interface Analytics {
  id: string
  analytics_request_id: string
  session_id: string
  generated_at: string
  ai_qualitative_output: AIQualitativeOutput
  ai_quantitative_output: AIQuantitativeOutput
  metric_quantitative_output: MetricQuantitativeOutput
  model_used: string | null
  analysis_duration_ms: number | null
}
