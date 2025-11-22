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
  refactoring_operations: number
  bulk_edit_operations: number
}

export interface TimeConsumptionMetrics {
  total_session_time_minutes: number
  average_session_length_minutes: number
  peak_hours: number[]
  break_duration_minutes: number
  context_switching_time_minutes: number
}

export interface TokenConsumptionMetrics {
  total_tokens_used: number
  input_tokens: number
  output_tokens: number
  token_efficiency: number
  tokens_per_hour: number
}

export interface ToolUsageMetrics {
  total_operations: number
  successful_operations: number
  failed_operations: number
  tool_distribution: Record<string, number>
  average_execution_time_ms: number
}

export interface QuantitativeInput {
  file_changes: FileChangeMetrics
  time_metrics: TimeConsumptionMetrics
  token_metrics: TokenConsumptionMetrics
  tool_usage: ToolUsageMetrics
}

// QualitativeInput contains a single raw JSON string representing the full chat session
export interface QualitativeInput {
  // Raw JSON string containing the full session transcript with embedded tool uses
  raw_session: string
}

// Types for parsing the raw_session JSON (optional, for display purposes)
export interface EmbeddedToolUse {
  tool_name: string
  input: string
  result?: string
  success?: boolean
}

export interface SessionTurn {
  turn_number: number
  role: string
  content: string
  tool_uses?: EmbeddedToolUse[]
}

export interface SessionTranscript {
  session_id: string
  total_turns: number
  turns: SessionTurn[]
}

export interface QuantitativeOutput {
  overall_score: number
  code_quality_score: number
  productivity_score: number
  efficiency_score: number
  collaboration_score: number
  learning_score: number
}

export interface Insight {
  title: string
  description: string
  category: string
  confidence: number
}

export interface GoodPattern {
  pattern_name: string
  description: string
  frequency: number
  impact: string
}

export interface ImprovementArea {
  area_name: string
  current_state: string
  suggested_improvement: string
  expected_impact: string
  priority: string
}

export interface Recommendation {
  title: string
  description: string
  impact_score: number
  implementation_difficulty: string
}

export interface LearningObservation {
  observation: string
  skill_area: string
  progress_indicator: string
  next_steps: string[]
}

// Summary of qualitative evaluation
export interface QualitativeEvaluationSummary {
  total_entries: number
  categories_evaluated: number
  entries_version: string
}

// AI-generated qualitative output from configurable entry-based analysis
export interface AIQualitativeOutput {
  // Dynamic entries based on qualitative_entries.json configuration
  // Key is the entry key (e.g., "insights"), value is an array of items
  entries: Record<string, unknown[]>
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

export interface SessionMetrics {
  total_sessions: number
  average_session_duration_minutes: number
  session_consistency_score: number
}

export interface ProcessedTokenMetrics {
  total_tokens: number
  tokens_per_hour: number
  input_output_ratio: number
  token_efficiency_score: number
  cost_estimate: number
}

export interface ProcessedCodeMetrics {
  net_lines_changed: number
  files_per_session: number
  lines_per_hour: number
  refactoring_ratio: number
  code_velocity: number
}

export interface TimeEfficiencyMetrics {
  productivity_score: number
  context_switching_cost: number
  deep_work_ratio: number
  time_utilization: number
}

export interface ProcessedQuantitativeOutput {
  session_metrics: SessionMetrics
  token_metrics: ProcessedTokenMetrics
  code_change_metrics: ProcessedCodeMetrics
  time_efficiency_metrics: TimeEfficiencyMetrics
}

export interface Scores {
  overall: number
  code_quality: number
  productivity: number
  efficiency: number
  collaboration: number
  learning: number
}

export interface Metrics {
  total_files_modified: number
  total_files_read: number
  lines_added: number
  lines_removed: number
  total_tokens_used: number
  session_duration_minutes: number
}

export interface Analytics {
  id: string
  analytics_request_id: string
  session_id: string
  generated_at: string
  scores: Scores
  metrics: Metrics
  // Note: quantitative_input and qualitative_input are not stored here
  // as they can be reconstructed from session_id
  qualitative_output: AIQualitativeOutput
  processed_output: ProcessedQuantitativeOutput
  // AI-generated quantitative output from rubric-based LLM-as-a-judge evaluation
  ai_quantitative_output: AIQuantitativeOutput
  model_used: string | null
  analysis_duration_ms: number | null
}
