pub mod analytics_service;
pub mod analysis_pipeline;
pub mod background;
pub mod google_ai;
pub mod import_service;
pub mod query_service;
pub mod retrospection_service;

pub use analytics_service::{
    AnalyticsService, DailyActivity, DurationStats, ExportFilters, ExportRequest, ExportResponse,
    MessageRoleDistribution, ProjectStats, ProviderStats, UsageInsights,
};
pub use background::{BackgroundOperation, BackgroundOperationManager, OperationResult, CancellationResult};
pub use google_ai::{GoogleAiClient, GoogleAiConfig, GoogleAiError, GenerateContentRequest, GenerateContentResponse};
pub use import_service::{
    BatchImportRequest, BatchImportResponse, ChatFile, ImportFileRequest, ImportFileResponse,
    ImportService, ScanRequest, ScanResponse,
};
pub use query_service::{
    DateRange, QueryService, SearchRequest, SearchResponse, SearchResult, SessionDetailRequest,
    SessionDetailResponse, SessionFilters, SessionSummary, SessionsQueryRequest,
    SessionsQueryResponse,
};
pub use retrospection_service::RetrospectionService;
pub use analysis_pipeline::{AnalysisPipeline, AnalysisData, SessionMetrics};
