'use client'

import { ExclamationTriangleIcon, ReloadIcon } from '@radix-ui/react-icons'
import {
  Activity,
  Brain,
  CheckCircle2,
  Clock,
  Code,
  DollarSign,
  FileCode,
  Lightbulb,
  Sparkles,
  Target,
  Zap,
} from 'lucide-react'
import { useCallback, useEffect, useState } from 'react'
import {
  PolarAngleAxis,
  PolarGrid,
  PolarRadiusAxis,
  Radar,
  RadarChart,
  ResponsiveContainer,
  Tooltip,
} from 'recharts'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Progress } from '@/components/ui/progress'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { analyzeSession, getAnalysisResult } from '@/lib/api'
import type { Analytics } from '@/types'

interface AnalyticsPanelProps {
  sessionId: string
}

export function AnalyticsPanel({ sessionId }: AnalyticsPanelProps) {
  const [analytics, setAnalytics] = useState<Analytics | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const loadAnalytics = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const request = await analyzeSession(sessionId)
      if (request.status === 'completed') {
        const result = await getAnalysisResult(request.id)
        setAnalytics(result)
      } else if (request.status === 'failed') {
        setError(request.error_message || 'Analysis failed')
      }
    } catch (err) {
      console.error('[v0] Failed to load analytics:', err)
      setError('Failed to generate analytics')
    } finally {
      setLoading(false)
    }
  }, [sessionId])

  useEffect(() => {
    loadAnalytics()
  }, [loadAnalytics])

  if (loading) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-center space-y-4">
          <ReloadIcon className="w-8 h-8 animate-spin mx-auto text-primary" />
          <p className="text-muted-foreground">Analyzing session...</p>
        </div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-center space-y-4">
          <ExclamationTriangleIcon className="w-12 h-12 mx-auto text-destructive" />
          <p className="text-muted-foreground">{error}</p>
          <Button onClick={loadAnalytics}>Retry</Button>
        </div>
      </div>
    )
  }

  if (!analytics) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <p className="text-muted-foreground">No analytics available</p>
      </div>
    )
  }

  return (
    <div className="flex-1 overflow-y-auto min-h-0">
      <div className="p-6 max-w-7xl mx-auto space-y-6">
        {/* Performance Scores */}
        <Card className="bg-card/50 backdrop-blur-sm">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Target className="w-5 h-5" />
              Performance Scores
            </CardTitle>
          </CardHeader>
          <CardContent>
            <ResponsiveContainer width="100%" height={400}>
              <RadarChart
                data={[
                  { metric: 'Overall', score: analytics.scores.overall * 100 },
                  { metric: 'Code Quality', score: analytics.scores.code_quality * 100 },
                  { metric: 'Productivity', score: analytics.scores.productivity * 100 },
                  { metric: 'Efficiency', score: analytics.scores.efficiency * 100 },
                  { metric: 'Collaboration', score: analytics.scores.collaboration * 100 },
                  { metric: 'Learning', score: analytics.scores.learning * 100 },
                ]}
              >
                <PolarGrid stroke="#6366f1" strokeWidth={1.5} opacity={0.3} />
                <PolarAngleAxis
                  dataKey="metric"
                  tick={{ fill: '#e2e8f0', fontSize: 14, fontWeight: 500 }}
                  stroke="#6366f1"
                  strokeWidth={1}
                />
                <PolarRadiusAxis
                  angle={90}
                  domain={[0, 100]}
                  tick={{ fill: '#cbd5e1', fontSize: 12 }}
                  stroke="#6366f1"
                  strokeWidth={1}
                  opacity={0.5}
                />
                <Radar
                  name="Score"
                  dataKey="score"
                  stroke="#8b5cf6"
                  strokeWidth={3}
                  fill="#8b5cf6"
                  fillOpacity={0.25}
                  dot={{ fill: '#8b5cf6', r: 5, strokeWidth: 2, stroke: '#ffffff' }}
                />
                <Tooltip
                  contentStyle={{
                    backgroundColor: 'hsl(var(--popover))',
                    border: '1px solid hsl(var(--border))',
                    borderRadius: '8px',
                    color: 'hsl(var(--popover-foreground))',
                  }}
                  formatter={(value: number) => [`${value.toFixed(0)}%`, 'Score']}
                />
              </RadarChart>
            </ResponsiveContainer>
          </CardContent>
        </Card>

        {/* Quick Metrics */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium flex items-center gap-2">
                <FileCode className="w-4 h-4 text-primary" />
                Code Changes
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-2">
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Files Modified</span>
                <span className="font-semibold">{analytics.metrics.total_files_modified}</span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Lines Added</span>
                <span className="font-semibold text-green-500">
                  +{analytics.metrics.lines_added}
                </span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Lines Removed</span>
                <span className="font-semibold text-red-500">
                  -{analytics.metrics.lines_removed}
                </span>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium flex items-center gap-2">
                <Clock className="w-4 h-4 text-primary" />
                Time Metrics
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-2">
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Duration</span>
                <span className="font-semibold">
                  {analytics.metrics.session_duration_minutes} min
                </span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Deep Work Ratio</span>
                <span className="font-semibold">
                  {(
                    analytics.processed_output.time_efficiency_metrics.deep_work_ratio * 100
                  ).toFixed(0)}
                  %
                </span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Time Utilization</span>
                <span className="font-semibold">
                  {(
                    analytics.processed_output.time_efficiency_metrics.time_utilization * 100
                  ).toFixed(0)}
                  %
                </span>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium flex items-center gap-2">
                <Zap className="w-4 h-4 text-primary" />
                Token Usage
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-2">
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Total Tokens</span>
                <span className="font-semibold">
                  {analytics.metrics.total_tokens_used.toLocaleString()}
                </span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Tokens/Hour</span>
                <span className="font-semibold">
                  {analytics.processed_output.token_metrics.tokens_per_hour.toLocaleString()}
                </span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Efficiency</span>
                <span className="font-semibold">
                  {(analytics.processed_output.token_metrics.token_efficiency_score * 100).toFixed(
                    0
                  )}
                  %
                </span>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium flex items-center gap-2">
                <Code className="w-4 h-4 text-primary" />
                Code Velocity
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-2">
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Lines/Hour</span>
                <span className="font-semibold">
                  {analytics.processed_output.code_change_metrics.lines_per_hour.toFixed(0)}
                </span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Velocity Score</span>
                <span className="font-semibold">
                  {(analytics.processed_output.code_change_metrics.code_velocity * 100).toFixed(0)}%
                </span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Refactoring Ratio</span>
                <span className="font-semibold">
                  {(analytics.processed_output.code_change_metrics.refactoring_ratio * 100).toFixed(
                    0
                  )}
                  %
                </span>
              </div>
            </CardContent>
          </Card>
        </div>

        {/* Detailed Analytics Tabs */}
        <Tabs defaultValue="insights" className="w-full">
          <TabsList className="grid w-full grid-cols-6">
            <TabsTrigger value="insights">Insights</TabsTrigger>
            <TabsTrigger value="patterns">Patterns</TabsTrigger>
            <TabsTrigger value="improvements">Improvements</TabsTrigger>
            <TabsTrigger value="recommendations">Recommendations</TabsTrigger>
            <TabsTrigger value="learning">Learning</TabsTrigger>
            <TabsTrigger value="details">Details</TabsTrigger>
          </TabsList>

          <TabsContent value="insights" className="space-y-4 mt-4">
            {analytics.qualitative_output.insights.map((insight, idx) => (
              <Card key={`${insight.title}-${idx}`}>
                <CardHeader>
                  <div className="flex items-start justify-between">
                    <div className="flex items-start gap-3">
                      <Lightbulb className="w-5 h-5 text-primary mt-0.5" />
                      <div>
                        <CardTitle className="text-base">{insight.title}</CardTitle>
                        <Badge variant="outline" className="mt-2">
                          {insight.category}
                        </Badge>
                      </div>
                    </div>
                    <Badge variant="secondary">
                      {Math.round(insight.confidence * 100)}% confidence
                    </Badge>
                  </div>
                </CardHeader>
                <CardContent>
                  <p className="text-sm text-muted-foreground leading-relaxed">
                    {insight.description}
                  </p>
                </CardContent>
              </Card>
            ))}
          </TabsContent>

          <TabsContent value="patterns" className="space-y-4 mt-4">
            {analytics.qualitative_output.good_patterns.map((pattern, idx) => (
              <Card key={`${pattern.pattern_name}-${idx}`}>
                <CardHeader>
                  <div className="flex items-start justify-between">
                    <div className="flex items-start gap-3">
                      <CheckCircle2 className="w-5 h-5 text-green-500 mt-0.5" />
                      <CardTitle className="text-base">{pattern.pattern_name}</CardTitle>
                    </div>
                    <Badge variant="secondary">{pattern.frequency}x</Badge>
                  </div>
                </CardHeader>
                <CardContent className="space-y-2">
                  <p className="text-sm text-muted-foreground leading-relaxed">
                    {pattern.description}
                  </p>
                  <div className="flex items-center gap-2 text-sm">
                    <span className="text-muted-foreground">Impact:</span>
                    <Badge variant="outline">{pattern.impact}</Badge>
                  </div>
                </CardContent>
              </Card>
            ))}
          </TabsContent>

          <TabsContent value="improvements" className="space-y-4 mt-4">
            {analytics.qualitative_output.improvement_areas.map((area, idx) => (
              <Card key={`${area.area_name}-${idx}`}>
                <CardHeader>
                  <div className="flex items-start justify-between">
                    <div className="flex items-start gap-3">
                      <ExclamationTriangleIcon className="w-5 h-5 text-orange-500 mt-0.5" />
                      <CardTitle className="text-base">{area.area_name}</CardTitle>
                    </div>
                    <Badge
                      variant={
                        area.priority === 'high'
                          ? 'destructive'
                          : area.priority === 'medium'
                            ? 'default'
                            : 'secondary'
                      }
                    >
                      {area.priority}
                    </Badge>
                  </div>
                </CardHeader>
                <CardContent className="space-y-3">
                  <div>
                    <span className="text-sm font-medium">Current State:</span>
                    <p className="text-sm text-muted-foreground mt-1">{area.current_state}</p>
                  </div>
                  <div>
                    <span className="text-sm font-medium">Suggestion:</span>
                    <p className="text-sm text-muted-foreground mt-1">
                      {area.suggested_improvement}
                    </p>
                  </div>
                  <div>
                    <span className="text-sm font-medium">Expected Impact:</span>
                    <p className="text-sm text-muted-foreground mt-1">{area.expected_impact}</p>
                  </div>
                </CardContent>
              </Card>
            ))}
          </TabsContent>

          <TabsContent value="recommendations" className="space-y-4 mt-4">
            {analytics.qualitative_output.recommendations.map((rec, idx) => (
              <Card key={`${rec.title}-${idx}`}>
                <CardHeader>
                  <div className="flex items-start justify-between">
                    <div className="flex items-start gap-3">
                      <Sparkles className="w-5 h-5 text-primary mt-0.5" />
                      <CardTitle className="text-base">{rec.title}</CardTitle>
                    </div>
                    <div className="flex items-center gap-2">
                      <Badge variant="outline">{rec.implementation_difficulty}</Badge>
                      <Badge variant="secondary">Impact: {rec.impact_score}/10</Badge>
                    </div>
                  </div>
                </CardHeader>
                <CardContent>
                  <p className="text-sm text-muted-foreground leading-relaxed">{rec.description}</p>
                </CardContent>
              </Card>
            ))}
          </TabsContent>

          <TabsContent value="learning" className="space-y-4 mt-4">
            {analytics.qualitative_output.learning_observations.map((obs, idx) => (
              <Card key={`${obs.skill_area}-${idx}`}>
                <CardHeader>
                  <div className="flex items-start gap-3">
                    <Brain className="w-5 h-5 text-primary mt-0.5" />
                    <div className="flex-1">
                      <CardTitle className="text-base mb-2">{obs.skill_area}</CardTitle>
                      <Badge variant="outline">{obs.progress_indicator}</Badge>
                    </div>
                  </div>
                </CardHeader>
                <CardContent className="space-y-3">
                  <div>
                    <span className="text-sm font-medium">Observation:</span>
                    <p className="text-sm text-muted-foreground mt-1">{obs.observation}</p>
                  </div>
                  <div>
                    <span className="text-sm font-medium">Next Steps:</span>
                    <ul className="mt-1 space-y-1">
                      {obs.next_steps.map((step, i) => (
                        <li
                          key={`${step.substring(0, 20)}-${i}`}
                          className="text-sm text-muted-foreground flex items-start gap-2"
                        >
                          <span className="text-primary mt-1">•</span>
                          <span>{step}</span>
                        </li>
                      ))}
                    </ul>
                  </div>
                </CardContent>
              </Card>
            ))}
          </TabsContent>

          <TabsContent value="details" className="space-y-6 mt-4">
            {/* Tool Usage Section */}
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Activity className="w-5 h-5" />
                  Tool Usage Metrics
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                  <div>
                    <div className="text-2xl font-bold">
                      {analytics.quantitative_input.tool_usage.total_operations}
                    </div>
                    <div className="text-sm text-muted-foreground">Total Operations</div>
                  </div>
                  <div>
                    <div className="text-2xl font-bold text-green-500">
                      {analytics.quantitative_input.tool_usage.successful_operations}
                    </div>
                    <div className="text-sm text-muted-foreground">Successful</div>
                  </div>
                  <div>
                    <div className="text-2xl font-bold text-red-500">
                      {analytics.quantitative_input.tool_usage.failed_operations}
                    </div>
                    <div className="text-sm text-muted-foreground">Failed</div>
                  </div>
                  <div>
                    <div className="text-2xl font-bold">
                      {analytics.quantitative_input.tool_usage.average_execution_time_ms}ms
                    </div>
                    <div className="text-sm text-muted-foreground">Avg. Execution</div>
                  </div>
                </div>

                <div className="space-y-2">
                  <div className="text-sm font-medium">Tool Distribution:</div>
                  {Object.entries(analytics.quantitative_input.tool_usage.tool_distribution).map(
                    ([tool, count]) => (
                      <div key={tool} className="flex items-center justify-between">
                        <span className="text-sm text-muted-foreground">{tool}</span>
                        <div className="flex items-center gap-2">
                          <Progress
                            value={
                              (count / analytics.quantitative_input.tool_usage.total_operations) *
                              100
                            }
                            className="w-24 h-2"
                          />
                          <span className="text-sm font-semibold w-8 text-right">{count}</span>
                        </div>
                      </div>
                    )
                  )}
                </div>
              </CardContent>
            </Card>

            {/* Project Context Section */}
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <FileCode className="w-5 h-5" />
                  Project Context
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                  <div>
                    <div className="text-sm font-medium mb-1">Project Type</div>
                    <Badge variant="outline">
                      {analytics.qualitative_input.project_context.project_type}
                    </Badge>
                  </div>
                  <div>
                    <div className="text-sm font-medium mb-1">Development Stage</div>
                    <Badge variant="outline">
                      {analytics.qualitative_input.project_context.development_stage}
                    </Badge>
                  </div>
                </div>
                <div>
                  <div className="text-sm font-medium mb-2">Technology Stack</div>
                  <div className="flex flex-wrap gap-2">
                    {analytics.qualitative_input.project_context.technology_stack.map((tech, i) => (
                      <Badge key={`${tech}-${i}`} variant="secondary">
                        {tech}
                      </Badge>
                    ))}
                  </div>
                </div>
                <div>
                  <div className="text-sm font-medium mb-2">Complexity Score</div>
                  <div className="flex items-center gap-3">
                    <Progress
                      value={analytics.qualitative_input.project_context.project_complexity * 10}
                      className="flex-1"
                    />
                    <span className="text-sm font-semibold">
                      {analytics.qualitative_input.project_context.project_complexity}/10
                    </span>
                  </div>
                </div>
              </CardContent>
            </Card>

            {/* Chat Context Section */}
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Brain className="w-5 h-5" />
                  Conversation Analysis
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <div>
                  <div className="text-sm font-medium mb-2">Conversation Flow</div>
                  <p className="text-sm text-muted-foreground">
                    {analytics.qualitative_input.chat_context.conversation_flow}
                  </p>
                </div>
                <div>
                  <div className="text-sm font-medium mb-2">AI Interaction Quality</div>
                  <div className="flex items-center gap-3">
                    <Progress
                      value={analytics.qualitative_input.chat_context.ai_interaction_quality * 10}
                      className="flex-1"
                    />
                    <span className="text-sm font-semibold">
                      {analytics.qualitative_input.chat_context.ai_interaction_quality}/10
                    </span>
                  </div>
                </div>
                <div>
                  <div className="text-sm font-medium mb-2">Problem Solving Patterns</div>
                  <div className="flex flex-wrap gap-2">
                    {analytics.qualitative_input.chat_context.problem_solving_patterns.map(
                      (pattern, i) => (
                        <Badge key={`${pattern}-${i}`} variant="outline">
                          {pattern}
                        </Badge>
                      )
                    )}
                  </div>
                </div>
                <div>
                  <div className="text-sm font-medium mb-2">Key Topics</div>
                  <div className="flex flex-wrap gap-2">
                    {analytics.qualitative_input.chat_context.key_topics.map((topic, i) => (
                      <Badge key={`${topic}-${i}`} variant="secondary">
                        {topic}
                      </Badge>
                    ))}
                  </div>
                </div>
              </CardContent>
            </Card>

            {/* Cost Estimate Section */}
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <DollarSign className="w-5 h-5" />
                  Cost Analysis
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="grid grid-cols-2 md:grid-cols-3 gap-4">
                  <div>
                    <div className="text-2xl font-bold">
                      ${analytics.processed_output.token_metrics.cost_estimate.toFixed(4)}
                    </div>
                    <div className="text-sm text-muted-foreground">Estimated Cost</div>
                  </div>
                  <div>
                    <div className="text-2xl font-bold">
                      {analytics.quantitative_input.token_metrics.input_tokens.toLocaleString()}
                    </div>
                    <div className="text-sm text-muted-foreground">Input Tokens</div>
                  </div>
                  <div>
                    <div className="text-2xl font-bold">
                      {analytics.quantitative_input.token_metrics.output_tokens.toLocaleString()}
                    </div>
                    <div className="text-sm text-muted-foreground">Output Tokens</div>
                  </div>
                </div>
              </CardContent>
            </Card>
          </TabsContent>
        </Tabs>

        {/* Footer Info */}
        <div className="text-center text-xs text-muted-foreground pt-4 border-t border-border">
          {analytics.model_used && <span>Analyzed using {analytics.model_used}</span>}
          {analytics.analysis_duration_ms && (
            <span> • Completed in {(analytics.analysis_duration_ms / 1000).toFixed(2)}s</span>
          )}
        </div>
      </div>
    </div>
  )
}
