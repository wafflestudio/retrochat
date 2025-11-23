import { invoke } from '@tauri-apps/api/core'
import type {
  Analytics,
  AnalyticsRequest,
  SearchResult,
  Session,
  SessionWithMessages,
} from '@/types'

const MOCK_SESSIONS: Session[] = [
  {
    id: '550e8400-e29b-41d4-a716-446655440001',
    provider: 'claude',
    project_name: 'Customer Support Dashboard',
    message_count: 24,
    created_at: '2024-01-15T09:30:00Z',
    updated_at: '2024-01-15T11:45:00Z',
  },
  {
    id: '550e8400-e29b-41d4-a716-446655440002',
    provider: 'gpt4',
    project_name: 'Code Review Session',
    message_count: 18,
    created_at: '2024-01-14T14:20:00Z',
    updated_at: '2024-01-14T16:10:00Z',
  },
  {
    id: '550e8400-e29b-41d4-a716-446655440003',
    provider: 'claude',
    project_name: 'Blog Post Generation',
    message_count: 12,
    created_at: '2024-01-13T10:00:00Z',
    updated_at: '2024-01-13T11:30:00Z',
  },
  {
    id: '550e8400-e29b-41d4-a716-446655440004',
    provider: 'gpt4',
    project_name: 'Learning TypeScript',
    message_count: 35,
    created_at: '2024-01-12T08:15:00Z',
    updated_at: '2024-01-12T12:00:00Z',
  },
  {
    id: '550e8400-e29b-41d4-a716-446655440005',
    provider: 'gemini',
    project_name: 'Data Analysis Task',
    message_count: 16,
    created_at: '2024-01-11T15:45:00Z',
    updated_at: '2024-01-11T17:20:00Z',
  },
  {
    id: '550e8400-e29b-41d4-a716-446655440006',
    provider: 'claude',
    project_name: 'Email Draft Assistant',
    message_count: 8,
    created_at: '2024-01-10T13:30:00Z',
    updated_at: '2024-01-10T14:15:00Z',
  },
]

const MOCK_SESSION_DETAILS: Record<string, SessionWithMessages> = {
  '550e8400-e29b-41d4-a716-446655440001': {
    ...MOCK_SESSIONS[0],
    messages: [
      {
        id: 'msg-001',
        role: 'user',
        content: 'I need help building a customer support dashboard',
        timestamp: '2024-01-15T09:30:00Z',
        message_type: 'SimpleMessage',
        tool_operation: null,
      },
      {
        id: 'msg-002-thinking',
        role: 'assistant',
        content:
          "To build an effective customer support dashboard, I need to consider:\n1. Key metrics to display (response time, ticket volume, customer satisfaction)\n2. Real-time data updates\n3. User-friendly interface with intuitive navigation\n4. Integration with existing support systems\n\nI'll start by creating a modular component structure that can be easily extended.",
        timestamp: '2024-01-15T09:30:15Z',
        message_type: 'Thinking',
        tool_operation: null,
      },
      {
        id: 'msg-002',
        role: 'assistant',
        content:
          "I'll help you build a customer support dashboard. Let me create the main components.",
        timestamp: '2024-01-15T09:30:30Z',
        message_type: 'SimpleMessage',
        tool_operation: null,
      },
      {
        id: 'msg-003-request',
        role: 'assistant',
        content: 'Creating dashboard layout component...',
        timestamp: '2024-01-15T09:31:00Z',
        message_type: 'ToolRequest',
        tool_operation: {
          id: 'tool-001',
          tool_use_id: 'use-001',
          tool_name: 'write_file',
          timestamp: '2024-01-15T09:31:00Z',
          success: null,
          result_summary: null,
          file_metadata: {
            file_path: 'components/dashboard-layout.tsx',
            file_extension: 'tsx',
            is_code_file: true,
            lines_added: null,
            lines_removed: null,
          },
          bash_metadata: null,
        },
      },
      {
        id: 'msg-003-result',
        role: 'assistant',
        content: 'Successfully created dashboard layout component',
        timestamp: '2024-01-15T09:31:05Z',
        message_type: 'ToolResult',
        tool_operation: {
          id: 'tool-001',
          tool_use_id: 'use-001',
          tool_name: 'write_file',
          timestamp: '2024-01-15T09:31:05Z',
          success: true,
          result_summary: 'Created dashboard layout component with responsive design',
          file_metadata: {
            file_path: 'components/dashboard-layout.tsx',
            file_extension: 'tsx',
            is_code_file: true,
            lines_added: 145,
            lines_removed: 0,
          },
          bash_metadata: null,
        },
      },
      {
        id: 'msg-004',
        role: 'user',
        content: 'Can you add a metrics section with charts?',
        timestamp: '2024-01-15T09:32:00Z',
        message_type: 'SimpleMessage',
        tool_operation: null,
      },
      {
        id: 'msg-005-thinking',
        role: 'assistant',
        content:
          "For the metrics section, I should:\n- Use a charting library like Recharts for data visualization\n- Create reusable chart components\n- Design responsive card layouts\n- Include key performance indicators\n\nI'll integrate shadcn/ui charts which are built on top of Recharts for a consistent design system.",
        timestamp: '2024-01-15T09:32:15Z',
        message_type: 'Thinking',
        tool_operation: null,
      },
      {
        id: 'msg-005-request',
        role: 'assistant',
        content: 'Adding metrics component with charts...',
        timestamp: '2024-01-15T09:32:30Z',
        message_type: 'ToolRequest',
        tool_operation: {
          id: 'tool-002',
          tool_use_id: 'use-002',
          tool_name: 'write_file',
          timestamp: '2024-01-15T09:32:30Z',
          success: null,
          result_summary: null,
          file_metadata: {
            file_path: 'components/metrics-section.tsx',
            file_extension: 'tsx',
            is_code_file: true,
            lines_added: null,
            lines_removed: null,
          },
          bash_metadata: null,
        },
      },
      {
        id: 'msg-005-result',
        role: 'assistant',
        content: 'Metrics component created successfully',
        timestamp: '2024-01-15T09:32:45Z',
        message_type: 'ToolResult',
        tool_operation: {
          id: 'tool-002',
          tool_use_id: 'use-002',
          tool_name: 'write_file',
          timestamp: '2024-01-15T09:32:45Z',
          success: true,
          result_summary: 'Created metrics component with recharts integration',
          file_metadata: {
            file_path: 'components/metrics-section.tsx',
            file_extension: 'tsx',
            is_code_file: true,
            lines_added: 98,
            lines_removed: 0,
          },
          bash_metadata: null,
        },
      },
      {
        id: 'msg-006',
        role: 'user',
        content: 'The charts look great! Can you also add real-time data updates?',
        timestamp: '2024-01-15T09:35:00Z',
        message_type: 'SimpleMessage',
        tool_operation: null,
      },
      {
        id: 'msg-007-thinking',
        role: 'assistant',
        content:
          "For real-time updates, I need to:\n1. Implement WebSocket or polling mechanism\n2. Add state management for real-time data\n3. Optimize re-renders to prevent performance issues\n4. Add loading states and error handling\n\nI'll use React hooks with SWR for efficient data fetching and caching.",
        timestamp: '2024-01-15T09:35:15Z',
        message_type: 'Thinking',
        tool_operation: null,
      },
      {
        id: 'msg-007-request',
        role: 'assistant',
        content: 'Setting up real-time data hooks...',
        timestamp: '2024-01-15T09:35:30Z',
        message_type: 'ToolRequest',
        tool_operation: {
          id: 'tool-003',
          tool_use_id: 'use-003',
          tool_name: 'write_file',
          timestamp: '2024-01-15T09:35:30Z',
          success: null,
          result_summary: null,
          file_metadata: {
            file_path: 'hooks/use-realtime-data.ts',
            file_extension: 'ts',
            is_code_file: true,
            lines_added: null,
            lines_removed: null,
          },
          bash_metadata: null,
        },
      },
      {
        id: 'msg-007-result',
        role: 'assistant',
        content: 'Real-time data hook created successfully',
        timestamp: '2024-01-15T09:35:50Z',
        message_type: 'ToolResult',
        tool_operation: {
          id: 'tool-003',
          tool_use_id: 'use-003',
          tool_name: 'write_file',
          timestamp: '2024-01-15T09:35:50Z',
          success: true,
          result_summary: 'Implemented custom hook with SWR for real-time data updates',
          file_metadata: {
            file_path: 'hooks/use-realtime-data.ts',
            file_extension: 'ts',
            is_code_file: true,
            lines_added: 67,
            lines_removed: 0,
          },
          bash_metadata: null,
        },
      },
      {
        id: 'msg-008-request',
        role: 'assistant',
        content: 'Updating metrics section to use real-time data...',
        timestamp: '2024-01-15T09:36:00Z',
        message_type: 'ToolRequest',
        tool_operation: {
          id: 'tool-004',
          tool_use_id: 'use-004',
          tool_name: 'edit_file',
          timestamp: '2024-01-15T09:36:00Z',
          success: null,
          result_summary: null,
          file_metadata: {
            file_path: 'components/metrics-section.tsx',
            file_extension: 'tsx',
            is_code_file: true,
            lines_added: null,
            lines_removed: null,
          },
          bash_metadata: null,
        },
      },
      {
        id: 'msg-008-result',
        role: 'assistant',
        content: 'Metrics section updated with real-time capabilities',
        timestamp: '2024-01-15T09:36:20Z',
        message_type: 'ToolResult',
        tool_operation: {
          id: 'tool-004',
          tool_use_id: 'use-004',
          tool_name: 'edit_file',
          timestamp: '2024-01-15T09:36:20Z',
          success: true,
          result_summary: 'Integrated real-time hook with automatic data refresh every 5 seconds',
          file_metadata: {
            file_path: 'components/metrics-section.tsx',
            file_extension: 'tsx',
            is_code_file: true,
            lines_added: 23,
            lines_removed: 8,
          },
          bash_metadata: null,
        },
      },
      {
        id: 'msg-009',
        role: 'assistant',
        content:
          "Perfect! I've added real-time data updates to your dashboard. The metrics will now automatically refresh every 5 seconds using an efficient polling mechanism with SWR.",
        timestamp: '2024-01-15T09:36:30Z',
        message_type: 'SimpleMessage',
        tool_operation: null,
      },
      {
        id: 'msg-010',
        role: 'user',
        content: 'This is excellent! Can we also add a search feature to filter tickets?',
        timestamp: '2024-01-15T09:40:00Z',
        message_type: 'SimpleMessage',
        tool_operation: null,
      },
      {
        id: 'msg-011-thinking',
        role: 'assistant',
        content:
          'For the search feature:\n- Add a search input component with debouncing\n- Implement filtering logic on the frontend\n- Consider adding backend search for large datasets\n- Add keyboard shortcuts for better UX\n- Include filter chips to show active filters',
        timestamp: '2024-01-15T09:40:10Z',
        message_type: 'Thinking',
        tool_operation: null,
      },
      {
        id: 'msg-011-request',
        role: 'assistant',
        content: 'Creating search and filter component...',
        timestamp: '2024-01-15T09:40:30Z',
        message_type: 'ToolRequest',
        tool_operation: {
          id: 'tool-005',
          tool_use_id: 'use-005',
          tool_name: 'write_file',
          timestamp: '2024-01-15T09:40:30Z',
          success: null,
          result_summary: null,
          file_metadata: {
            file_path: 'components/ticket-search.tsx',
            file_extension: 'tsx',
            is_code_file: true,
            lines_added: null,
            lines_removed: null,
          },
          bash_metadata: null,
        },
      },
      {
        id: 'msg-011-result-failed',
        role: 'assistant',
        content: 'Failed to create search component',
        timestamp: '2024-01-15T09:40:45Z',
        message_type: 'ToolResult',
        tool_operation: {
          id: 'tool-005',
          tool_use_id: 'use-005',
          tool_name: 'write_file',
          timestamp: '2024-01-15T09:40:45Z',
          success: false,
          result_summary: "Import error: Cannot find module 'use-debounce'",
          file_metadata: {
            file_path: 'components/ticket-search.tsx',
            file_extension: 'tsx',
            is_code_file: true,
            lines_added: 0,
            lines_removed: 0,
          },
          bash_metadata: null,
        },
      },
      {
        id: 'msg-012-thinking',
        role: 'assistant',
        content:
          "The component failed because we're missing the debounce utility. I'll create a custom debounce hook instead of relying on an external package. This will give us more control and reduce dependencies.",
        timestamp: '2024-01-15T09:41:00Z',
        message_type: 'Thinking',
        tool_operation: null,
      },
      {
        id: 'msg-012-request',
        role: 'assistant',
        content: 'Creating custom debounce hook...',
        timestamp: '2024-01-15T09:41:15Z',
        message_type: 'ToolRequest',
        tool_operation: {
          id: 'tool-006',
          tool_use_id: 'use-006',
          tool_name: 'write_file',
          timestamp: '2024-01-15T09:41:15Z',
          success: null,
          result_summary: null,
          file_metadata: {
            file_path: 'hooks/use-debounce.ts',
            file_extension: 'ts',
            is_code_file: true,
            lines_added: null,
            lines_removed: null,
          },
          bash_metadata: null,
        },
      },
      {
        id: 'msg-012-result',
        role: 'assistant',
        content: 'Debounce hook created',
        timestamp: '2024-01-15T09:41:25Z',
        message_type: 'ToolResult',
        tool_operation: {
          id: 'tool-006',
          tool_use_id: 'use-006',
          tool_name: 'write_file',
          timestamp: '2024-01-15T09:41:25Z',
          success: true,
          result_summary: 'Custom debounce hook with configurable delay',
          file_metadata: {
            file_path: 'hooks/use-debounce.ts',
            file_extension: 'ts',
            is_code_file: true,
            lines_added: 28,
            lines_removed: 0,
          },
          bash_metadata: null,
        },
      },
      {
        id: 'msg-013-request',
        role: 'assistant',
        content: 'Retrying search component creation...',
        timestamp: '2024-01-15T09:41:35Z',
        message_type: 'ToolRequest',
        tool_operation: {
          id: 'tool-007',
          tool_use_id: 'use-007',
          tool_name: 'write_file',
          timestamp: '2024-01-15T09:41:35Z',
          success: null,
          result_summary: null,
          file_metadata: {
            file_path: 'components/ticket-search.tsx',
            file_extension: 'tsx',
            is_code_file: true,
            lines_added: null,
            lines_removed: null,
          },
          bash_metadata: null,
        },
      },
      {
        id: 'msg-013-result',
        role: 'assistant',
        content: 'Search component created successfully',
        timestamp: '2024-01-15T09:41:50Z',
        message_type: 'ToolResult',
        tool_operation: {
          id: 'tool-007',
          tool_use_id: 'use-007',
          tool_name: 'write_file',
          timestamp: '2024-01-15T09:41:50Z',
          success: true,
          result_summary: 'Search component with debouncing and keyboard shortcuts',
          file_metadata: {
            file_path: 'components/ticket-search.tsx',
            file_extension: 'tsx',
            is_code_file: true,
            lines_added: 112,
            lines_removed: 0,
          },
          bash_metadata: null,
        },
      },
      {
        id: 'msg-014',
        role: 'assistant',
        content:
          "Great! I've added a comprehensive search feature with debouncing for better performance and keyboard shortcuts (Cmd+K / Ctrl+K) for quick access.",
        timestamp: '2024-01-15T09:42:00Z',
        message_type: 'SimpleMessage',
        tool_operation: null,
      },
    ],
  },
}

const MOCK_ANALYTICS_REQUESTS: AnalyticsRequest[] = [
  {
    id: 'req-001',
    session_id: '550e8400-e29b-41d4-a716-446655440001',
    status: 'completed',
    started_at: '2024-01-15T12:00:00Z',
    completed_at: '2024-01-15T12:00:45Z',
    created_by: null,
    error_message: null,
  },
  {
    id: 'req-002',
    session_id: '550e8400-e29b-41d4-a716-446655440002',
    status: 'completed',
    started_at: '2024-01-14T17:00:00Z',
    completed_at: '2024-01-14T17:01:12Z',
    created_by: null,
    error_message: null,
  },
  {
    id: 'req-003',
    session_id: '550e8400-e29b-41d4-a716-446655440003',
    status: 'running',
    started_at: '2024-01-13T12:30:00Z',
    completed_at: null,
    created_by: null,
    error_message: null,
  },
]

const MOCK_ANALYTICS: Record<string, Analytics> = {
  'req-001': {
    id: 'analytics-001',
    analytics_request_id: 'req-001',
    session_id: '550e8400-e29b-41d4-a716-446655440001',
    generated_at: '2024-01-15T12:00:45Z',
    ai_qualitative_output: {
      entries: [
        {
          key: 'insights',
          title: 'Insights',
          description: 'Key observations and patterns identified during the session',
          summary: 'Strong component architecture with clear separation of concerns',
          items: [
            'Demonstrated excellent component composition with reusable patterns',
            'Effective problem-solving approach by breaking down complex requirements',
            'Strong TypeScript typing throughout with proper interface definitions',
          ],
        },
        {
          key: 'good_patterns',
          title: 'Good Patterns',
          description: 'Positive coding patterns and practices observed',
          summary: 'Consistent use of reusable components and type safety',
          items: [
            'Component Reusability: Created reusable components leveraged across the application',
            'Type Safety: Proper TypeScript types and interfaces defined for all components',
            'Responsive Design: Mobile-first approach with proper breakpoints',
          ],
        },
        {
          key: 'improvement_areas',
          title: 'Improvement Areas',
          description: 'Areas where improvements could be made',
          summary: 'Error handling and performance optimization opportunities identified',
          items: [
            'Error Handling: Implement comprehensive error boundaries and user-friendly messages',
            'Performance: Add React.memo and useMemo for expensive computations',
          ],
        },
        {
          key: 'recommendations',
          title: 'Recommendations',
          description: 'Suggested actions for future improvement',
          summary: 'Focus on testing and documentation',
          items: [
            'Add unit tests for critical components to ensure reliability',
            'Implement code splitting to reduce initial bundle size',
            'Add JSDoc comments to document component props and behavior',
          ],
        },
      ],
      summary: {
        total_entries: 11,
        categories_evaluated: 4,
        entries_version: '1.0',
      },
      entries_version: '1.0',
    },
    ai_quantitative_output: {
      rubric_scores: [
        {
          rubric_id: 'rubric_001',
          rubric_name: 'Code Quality',
          score: 4.4,
          max_score: 5,
          reasoning:
            'Excellent component composition with clear separation of concerns and reusable patterns.',
        },
        {
          rubric_id: 'rubric_002',
          rubric_name: 'Productivity',
          score: 4.1,
          max_score: 5,
          reasoning:
            'Efficient problem-solving approach with good task breakdown and iterative development.',
        },
        {
          rubric_id: 'rubric_003',
          rubric_name: 'Efficiency',
          score: 3.95,
          max_score: 5,
          reasoning: 'Good token efficiency and time utilization with minimal context switching.',
        },
        {
          rubric_id: 'rubric_004',
          rubric_name: 'Collaboration',
          score: 4.55,
          max_score: 5,
          reasoning: 'Clear communication with well-structured dialogue and iterative refinement.',
        },
        {
          rubric_id: 'rubric_005',
          rubric_name: 'Learning',
          score: 3.8,
          max_score: 5,
          reasoning:
            'Growing proficiency with React hooks and state management patterns demonstrated.',
        },
      ],
      rubric_summary: {
        total_score: 20.8,
        max_score: 25,
        percentage: 83.2,
        rubrics_evaluated: 5,
        rubrics_version: '1.0',
      },
    },
    metric_quantitative_output: {
      file_changes: {
        total_files_modified: 8,
        total_files_read: 15,
        lines_added: 543,
        lines_removed: 127,
        net_code_growth: 416,
      },
      time_metrics: {
        total_session_time_minutes: 135,
        peak_hours: [14, 15, 16],
      },
      token_metrics: {
        total_tokens_used: 45320,
        input_tokens: 28500,
        output_tokens: 16820,
        token_efficiency: 0.89,
      },
      tool_usage: {
        total_operations: 42,
        successful_operations: 39,
        failed_operations: 3,
        tool_distribution: {
          write_file: 18,
          read_file: 15,
          edit_file: 6,
          list_files: 3,
        },
        average_execution_time_ms: 245,
      },
    },
    model_used: 'gemini-2.0-flash',
    analysis_duration_ms: 3456,
  },
  'req-002': {
    id: 'analytics-002',
    analytics_request_id: 'req-002',
    session_id: '550e8400-e29b-41d4-a716-446655440002',
    generated_at: '2024-01-14T17:01:12Z',
    ai_qualitative_output: {
      entries: [
        {
          key: 'insights',
          title: 'Insights',
          description: 'Key observations and patterns identified during the session',
          summary: 'Effective refactoring with systematic approach',
          items: [
            'Successfully improved code quality through systematic refactoring',
            'Focused code review with specific improvement suggestions',
          ],
        },
        {
          key: 'good_patterns',
          title: 'Good Patterns',
          description: 'Positive coding patterns and practices observed',
          summary: 'Comprehensive validation patterns',
          items: ['Input Validation: Comprehensive validation patterns with proper error messages'],
        },
        {
          key: 'improvement_areas',
          title: 'Improvement Areas',
          description: 'Areas where improvements could be made',
          summary: 'Test coverage needs improvement',
          items: ['Test Coverage: Add comprehensive test suite for validation functions'],
        },
        {
          key: 'recommendations',
          title: 'Recommendations',
          description: 'Suggested actions for future improvement',
          summary: 'Add integration tests',
          items: ['Consider adding integration tests to validate end-to-end workflows'],
        },
      ],
      summary: {
        total_entries: 5,
        categories_evaluated: 4,
        entries_version: '1.0',
      },
      entries_version: '1.0',
    },
    ai_quantitative_output: {
      rubric_scores: [
        {
          rubric_id: 'rubric_001',
          rubric_name: 'Code Quality',
          score: 4.1,
          max_score: 5,
          reasoning: 'Successfully improved code quality through systematic refactoring approach.',
        },
        {
          rubric_id: 'rubric_002',
          rubric_name: 'Productivity',
          score: 3.75,
          max_score: 5,
          reasoning: 'Good productivity with focused code review sessions.',
        },
        {
          rubric_id: 'rubric_003',
          rubric_name: 'Efficiency',
          score: 3.55,
          max_score: 5,
          reasoning: 'Reasonable efficiency with some room for optimization.',
        },
        {
          rubric_id: 'rubric_004',
          rubric_name: 'Collaboration',
          score: 4.25,
          max_score: 5,
          reasoning: 'Clear communication and specific improvement suggestions.',
        },
        {
          rubric_id: 'rubric_005',
          rubric_name: 'Learning',
          score: 4.0,
          max_score: 5,
          reasoning: 'Strong understanding of refactoring principles and patterns.',
        },
      ],
      rubric_summary: {
        total_score: 19.65,
        max_score: 25,
        percentage: 78.6,
        rubrics_evaluated: 5,
        rubrics_version: '1.0',
      },
    },
    metric_quantitative_output: {
      file_changes: {
        total_files_modified: 5,
        total_files_read: 12,
        lines_added: 324,
        lines_removed: 89,
        net_code_growth: 235,
      },
      time_metrics: {
        total_session_time_minutes: 110,
        peak_hours: [10, 11],
      },
      token_metrics: {
        total_tokens_used: 32100,
        input_tokens: 20000,
        output_tokens: 12100,
        token_efficiency: 0.85,
      },
      tool_usage: {
        total_operations: 28,
        successful_operations: 26,
        failed_operations: 2,
        tool_distribution: {
          write_file: 12,
          read_file: 11,
          edit_file: 4,
          list_files: 1,
        },
        average_execution_time_ms: 220,
      },
    },
    model_used: 'gemini-2.0-flash',
    analysis_duration_ms: 2890,
  },
}

const MOCK_SEARCH_RESULTS: SearchResult[] = [
  {
    session_id: '550e8400-e29b-41d4-a716-446655440001',
    role: 'user',
    provider: 'claude',
    content: 'I need help building a customer support dashboard',
    timestamp: '2024-01-15T09:30:00Z',
  },
  {
    session_id: '550e8400-e29b-41d4-a716-446655440001',
    role: 'assistant',
    provider: 'claude',
    content: "I'll help you build a customer support dashboard. Let me create the main components.",
    timestamp: '2024-01-15T09:30:30Z',
  },
]

/**
 * Get paginated list of chat sessions
 */
export async function getSessions(
  page = 1,
  pageSize = 20,
  provider: string | null = null
): Promise<Session[]> {
  try {
    return await invoke('get_sessions', { page, pageSize, provider })
  } catch (_error) {
    console.log('[v0] Using mock data for getSessions')
    let filtered = MOCK_SESSIONS
    if (provider) {
      filtered = filtered.filter((s) => s.provider === provider)
    }
    return filtered
  }
}

/**
 * Get detailed information about a specific session
 */
export async function getSessionDetail(sessionId: string): Promise<SessionWithMessages> {
  try {
    return await invoke('get_session_detail', { sessionId })
  } catch (_error) {
    console.log('[v0] Using mock data for getSessionDetail')
    return (
      MOCK_SESSION_DETAILS[sessionId] || {
        ...MOCK_SESSIONS[0],
        messages: [],
      }
    )
  }
}

/**
 * Search messages by content
 */
export async function searchMessages(query: string, limit = 50): Promise<SearchResult[]> {
  try {
    return await invoke('search_messages', { query, limit })
  } catch (_error) {
    console.log('[v0] Using mock data for searchMessages')
    return MOCK_SEARCH_RESULTS.filter((result) =>
      result.content.toLowerCase().includes(query.toLowerCase())
    )
  }
}

/**
 * Get list of available providers
 */
export async function getProviders(): Promise<string[]> {
  try {
    return await invoke('get_providers')
  } catch (_error) {
    console.log('[v0] Using mock data for getProviders')
    return ['Claude Code', 'Gemini CLI', 'Codex']
  }
}

/**
 * Analyze a session (creates and executes analysis in one call)
 */
export async function analyzeSession(
  sessionId: string,
  customPrompt?: string
): Promise<AnalyticsRequest> {
  try {
    return await invoke('analyze_session', { sessionId, customPrompt })
  } catch (_error) {
    console.log('[v0] Using mock data for analyzeSession')
    const request: AnalyticsRequest = {
      id: `req-${Date.now()}`,
      session_id: sessionId,
      status: 'completed',
      started_at: new Date().toISOString(),
      completed_at: new Date().toISOString(),
      created_by: null,
      error_message: null,
    }
    return request
  }
}

/**
 * Create a new analysis request for a session without executing it
 */
export async function createAnalysis(
  sessionId: string,
  customPrompt?: string
): Promise<AnalyticsRequest> {
  try {
    return await invoke('create_analysis', { sessionId, customPrompt })
  } catch (_error) {
    console.log('[v0] Using mock data for createAnalysis')
    const request: AnalyticsRequest = {
      id: `req-${Date.now()}`,
      session_id: sessionId,
      status: 'pending',
      started_at: new Date().toISOString(),
      completed_at: null,
      created_by: null,
      error_message: null,
    }
    return request
  }
}

/**
 * Execute an analysis request
 */
export async function runAnalysis(requestId: string): Promise<string> {
  try {
    return await invoke('run_analysis', { requestId })
  } catch (_error) {
    console.log('[v0] Using mock data for runAnalysis')
    return MOCK_ANALYTICS_REQUESTS[0]?.session_id || '550e8400-e29b-41d4-a716-446655440001'
  }
}

/**
 * Get the status of an analysis request
 */
export async function getAnalysisStatus(requestId: string): Promise<AnalyticsRequest> {
  try {
    return await invoke('get_analysis_status', { requestId })
  } catch (_error) {
    console.log('[v0] Using mock data for getAnalysisStatus')
    return (
      MOCK_ANALYTICS_REQUESTS.find((req) => req.id === requestId) ||
      MOCK_ANALYTICS_REQUESTS[0] || {
        id: requestId,
        session_id: '550e8400-e29b-41d4-a716-446655440001',
        status: 'completed',
        started_at: new Date().toISOString(),
        completed_at: new Date().toISOString(),
        created_by: null,
        error_message: null,
      }
    )
  }
}

/**
 * Get the result of a completed analysis
 */
export async function getAnalysisResult(requestId: string): Promise<Analytics | null> {
  try {
    return await invoke('get_analysis_result', { requestId })
  } catch (_error) {
    console.log('[v0] Using mock data for getAnalysisResult')
    return MOCK_ANALYTICS[requestId] || MOCK_ANALYTICS['req-001'] || null
  }
}

/**
 * List analysis requests
 */
export async function listAnalyses(
  sessionId?: string,
  limit?: number
): Promise<AnalyticsRequest[]> {
  try {
    return await invoke('list_analyses', { sessionId, limit })
  } catch (_error) {
    console.log('[v0] Using mock data for listAnalyses')
    let filtered = MOCK_ANALYTICS_REQUESTS
    if (sessionId) {
      filtered = filtered.filter((req) => req.session_id === sessionId)
    }
    if (limit) {
      filtered = filtered.slice(0, limit)
    }
    return filtered
  }
}

/**
 * Cancel a pending or running analysis request
 */
export async function cancelAnalysis(requestId: string): Promise<void> {
  try {
    return await invoke('cancel_analysis', { requestId })
  } catch (_error) {
    console.log('[v0] Using mock data for cancelAnalysis - no action taken')
  }
}
