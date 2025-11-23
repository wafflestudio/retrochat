'use client'

import {
  Activity,
  AlertCircle,
  Brain,
  Clock,
  Code,
  FileCode,
  ListChecks,
  Loader2,
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
          <Loader2 className="w-8 h-8 animate-spin mx-auto text-primary" />
          <p className="text-muted-foreground">Analyzing session...</p>
        </div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-center space-y-4">
          <AlertCircle className="w-12 h-12 mx-auto text-destructive" />
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

  const radarData = analytics.ai_quantitative_output.rubric_scores.map((rubric) => ({
    metric: rubric.rubric_name,
    score: (rubric.score / rubric.max_score) * 100,
  }))

  const qualitativeEntries = analytics.ai_qualitative_output.entries

  return (
    <div className="flex-1 overflow-y-auto min-h-0">
      <div className="p-6 max-w-7xl mx-auto space-y-6">
        {/* AI Quantitative Output - Rubric Scores */}
        <Card className="bg-card/50 backdrop-blur-sm">
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle className="flex items-center gap-2">
                <Target className="w-5 h-5" />
                Performance Scores (LLM-as-a-Judge)
              </CardTitle>
              {analytics.ai_quantitative_output.rubric_summary && (
                <Badge variant="outline" className="text-base font-semibold">
                  {analytics.ai_quantitative_output.rubric_summary.percentage.toFixed(1)}%
                  <span className="text-muted-foreground ml-1 font-normal">
                    ({analytics.ai_quantitative_output.rubric_summary.total_score}/
                    {analytics.ai_quantitative_output.rubric_summary.max_score})
                  </span>
                </Badge>
              )}
            </div>
            {analytics.ai_quantitative_output.rubric_summary && (
              <p className="text-sm text-muted-foreground mt-2">
                Evaluated {analytics.ai_quantitative_output.rubric_summary.rubrics_evaluated}{' '}
                rubrics ({analytics.ai_quantitative_output.rubric_summary.rubrics_version})
              </p>
            )}
          </CardHeader>
          <CardContent>
            <ResponsiveContainer width="100%" height={400}>
              <RadarChart data={radarData}>
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
                  dot={{
                    fill: '#8b5cf6',
                    r: 5,
                    strokeWidth: 2,
                    stroke: '#ffffff',
                  }}
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

            {/* Rubric Score Details */}
            <div className="mt-6 space-y-3">
              {analytics.ai_quantitative_output.rubric_scores.map((rubric) => (
                <div key={rubric.rubric_id} className="space-y-2">
                  <div className="flex items-center justify-between">
                    <span className="text-sm font-medium">{rubric.rubric_name}</span>
                    <Badge variant="secondary">
                      {rubric.score}/{rubric.max_score}
                    </Badge>
                  </div>
                  <Progress value={(rubric.score / rubric.max_score) * 100} className="h-2" />
                  {rubric.reasoning && (
                    <p className="text-xs text-muted-foreground leading-relaxed">
                      {rubric.reasoning}
                    </p>
                  )}
                </div>
              ))}
            </div>
          </CardContent>
        </Card>

        {/* Metric Quantitative Output */}
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
                <span className="font-semibold">
                  {analytics.metric_quantitative_output.file_changes.total_files_modified}
                </span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Files Read</span>
                <span className="font-semibold">
                  {analytics.metric_quantitative_output.file_changes.total_files_read}
                </span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Lines Added</span>
                <span className="font-semibold text-green-500">
                  +{analytics.metric_quantitative_output.file_changes.lines_added}
                </span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Lines Removed</span>
                <span className="font-semibold text-red-500">
                  -{analytics.metric_quantitative_output.file_changes.lines_removed}
                </span>
              </div>
              <div className="flex justify-between text-sm border-t pt-2 mt-2">
                <span className="text-muted-foreground">Net Growth</span>
                <span className="font-semibold">
                  {analytics.metric_quantitative_output.file_changes.net_code_growth}
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
                <span className="text-muted-foreground">Total Duration</span>
                <span className="font-semibold">
                  {analytics.metric_quantitative_output.time_metrics.total_session_time_minutes} min
                </span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Peak Hours</span>
                <span className="font-semibold">
                  {analytics.metric_quantitative_output.time_metrics.peak_hours.join(', ')}h
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
                  {analytics.metric_quantitative_output.token_metrics.total_tokens_used.toLocaleString()}
                </span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Input</span>
                <span className="font-semibold">
                  {analytics.metric_quantitative_output.token_metrics.input_tokens.toLocaleString()}
                </span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Output</span>
                <span className="font-semibold">
                  {analytics.metric_quantitative_output.token_metrics.output_tokens.toLocaleString()}
                </span>
              </div>
              <div className="flex justify-between text-sm border-t pt-2 mt-2">
                <span className="text-muted-foreground">Efficiency</span>
                <span className="font-semibold">
                  {(
                    analytics.metric_quantitative_output.token_metrics.token_efficiency * 100
                  ).toFixed(0)}
                  %
                </span>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium flex items-center gap-2">
                <Activity className="w-4 h-4 text-primary" />
                Tool Usage
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-2">
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Total Ops</span>
                <span className="font-semibold">
                  {analytics.metric_quantitative_output.tool_usage.total_operations}
                </span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Success</span>
                <span className="font-semibold text-green-500">
                  {analytics.metric_quantitative_output.tool_usage.successful_operations}
                </span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Failed</span>
                <span className="font-semibold text-red-500">
                  {analytics.metric_quantitative_output.tool_usage.failed_operations}
                </span>
              </div>
              <div className="flex justify-between text-sm border-t pt-2 mt-2">
                <span className="text-muted-foreground">Avg Exec Time</span>
                <span className="font-semibold">
                  {analytics.metric_quantitative_output.tool_usage.average_execution_time_ms}
                  ms
                </span>
              </div>
            </CardContent>
          </Card>
        </div>

        {/* Tool Distribution Chart */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Code className="w-5 h-5" />
              Tool Distribution
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            {Object.entries(analytics.metric_quantitative_output.tool_usage.tool_distribution).map(
              ([tool, count]) => (
                <div key={tool} className="flex items-center justify-between gap-4">
                  <span className="text-sm text-muted-foreground min-w-32">{tool}</span>
                  <div className="flex-1 flex items-center gap-2">
                    <Progress
                      value={
                        (count / analytics.metric_quantitative_output.tool_usage.total_operations) *
                        100
                      }
                      className="h-2"
                    />
                    <span className="text-sm font-semibold min-w-12 text-right">{count}</span>
                  </div>
                </div>
              )
            )}
          </CardContent>
        </Card>

        {/* AI Qualitative Output - Dynamic Entries */}
        {qualitativeEntries.length > 0 && (
          <Card>
            <CardHeader>
              <div className="flex items-center justify-between">
                <CardTitle className="flex items-center gap-2">
                  <Brain className="w-5 h-5" />
                  Qualitative Analysis
                </CardTitle>
                {analytics.ai_qualitative_output.summary && (
                  <Badge variant="outline">
                    {analytics.ai_qualitative_output.summary.total_entries} entries •{' '}
                    {analytics.ai_qualitative_output.summary.categories_evaluated} categories
                  </Badge>
                )}
              </div>
              {analytics.ai_qualitative_output.entries_version && (
                <p className="text-sm text-muted-foreground mt-2">
                  Version: {analytics.ai_qualitative_output.entries_version}
                </p>
              )}
            </CardHeader>
            <CardContent>
              <Tabs defaultValue={qualitativeEntries[0]?.key || 'entries'} className="w-full">
                <TabsList
                  className="grid w-full"
                  style={{
                    gridTemplateColumns: `repeat(${qualitativeEntries.length}, 1fr)`,
                  }}
                >
                  {qualitativeEntries.map((entry) => (
                    <TabsTrigger key={entry.key} value={entry.key} className="capitalize">
                      {entry.title}
                    </TabsTrigger>
                  ))}
                </TabsList>

                {qualitativeEntries.map((entryOutput) => (
                  <TabsContent
                    key={entryOutput.key}
                    value={entryOutput.key}
                    className="space-y-3 mt-4"
                  >
                    {/* Summary */}
                    {entryOutput.summary && (
                      <div className="p-3 bg-muted/50 rounded-lg border">
                        <p className="text-sm text-muted-foreground italic">
                          {entryOutput.summary}
                        </p>
                      </div>
                    )}
                    {/* Items */}
                    {entryOutput.items.map((item, idx) => (
                      <Card key={`${entryOutput.key}-item-${idx}`}>
                        <CardContent className="pt-4">
                          <div className="prose prose-sm prose-invert max-w-none">
                            <p className="text-sm leading-relaxed">{item}</p>
                          </div>
                        </CardContent>
                      </Card>
                    ))}
                  </TabsContent>
                ))}
              </Tabs>
            </CardContent>
          </Card>
        )}

        {/* Analysis Metadata */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <ListChecks className="w-5 h-5" />
              Analysis Metadata
            </CardTitle>
          </CardHeader>
          <CardContent className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div>
              <div className="text-sm text-muted-foreground">Generated At</div>
              <div className="font-semibold">
                {new Date(analytics.generated_at).toLocaleString()}
              </div>
            </div>
            <div>
              <div className="text-sm text-muted-foreground">Model Used</div>
              <div className="font-semibold">{analytics.model_used || 'N/A'}</div>
            </div>
            <div>
              <div className="text-sm text-muted-foreground">Analysis Duration</div>
              <div className="font-semibold">{analytics.analysis_duration_ms}ms</div>
            </div>
            <div>
              <div className="text-sm text-muted-foreground">Session ID</div>
              <div className="font-mono text-xs">{analytics.session_id.substring(0, 8)}...</div>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  )
}
