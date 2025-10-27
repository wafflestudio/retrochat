pub mod analytics;
pub mod analytics_service;
pub mod auto_detect;
pub mod embedding_service;
pub mod google_ai;
pub mod import_service;
pub mod parser_service;
pub mod query_service;
pub mod retrospection_service;
pub mod watch_service;

pub use analytics::{
    ChatContext, ComprehensiveAnalysis, DailyActivity, DurationStats, FileChangeMetrics,
    FileContext, GoodPattern, ImprovementArea, Insight, LearningObservation,
    MessageRoleDistribution, ProcessedCodeMetrics, ProcessedQuantitativeOutput,
    ProcessedTokenMetrics, ProjectContext, ProjectStats, ProviderStats, QualitativeInput,
    QualitativeOutput, QuantitativeInput, QuantitativeOutput, Recommendation, SessionMetrics,
    TimeConsumptionMetrics, TimeEfficiencyMetrics, TokenConsumptionMetrics, ToolUsageMetrics,
    UsageInsights,
};
pub use analytics_service::AnalyticsService;
pub use auto_detect::{AutoDetectService, DetectedProvider};
pub use embedding_service::{EmbeddingService, EMBEDDING_DIM};
pub use google_ai::{
    GenerateContentRequest, GenerateContentResponse, GoogleAiClient, GoogleAiConfig, GoogleAiError,
};
pub use import_service::{
    BatchImportRequest, BatchImportResponse, ChatFile, ImportFileRequest, ImportFileResponse,
    ImportService, ScanRequest, ScanResponse,
};
pub use parser_service::ParserService;
pub use query_service::{
    DateRange, MessageGroup, QueryService, SearchRequest, SearchResponse, SearchResult,
    SessionDetailRequest, SessionDetailResponse, SessionFilters, SessionSummary,
    SessionsQueryRequest, SessionsQueryResponse,
};
pub use retrospection_service::{
    AnalysisData, RetrospectionCleanupHandler, RetrospectionService,
    SessionMetrics as RetrospectionSessionMetrics,
};
pub use watch_service::{collect_provider_paths, detect_provider, watch_paths_for_changes};
