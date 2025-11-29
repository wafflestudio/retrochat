"use client";

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
} from "lucide-react";
import { useTheme } from "next-themes";
import { useCallback, useEffect, useState } from "react";
import {
  PolarAngleAxis,
  PolarGrid,
  PolarRadiusAxis,
  Radar,
  RadarChart,
} from "recharts";
import {
  ChartConfig,
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
} from "@/components/ui/chart";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Progress } from "@/components/ui/progress";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  analyzeSession,
  getAnalysisResult,
  getAnalysisStatus,
  listAnalyses,
} from "@/lib/api";
import type { Analytics, AnalyticsRequest } from "@/types";

interface AnalyticsPanelProps {
  sessionId: string;
}

export function AnalyticsPanel({ sessionId }: AnalyticsPanelProps) {
  const { theme } = useTheme();
  const [analytics, setAnalytics] = useState<Analytics | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showConfirmDialog, setShowConfirmDialog] = useState(false);
  const [analyzing, setAnalyzing] = useState(false);
  const [currentRequest, setCurrentRequest] = useState<AnalyticsRequest | null>(
    null
  );

  const isDark = theme === "dark";

  // Chart configuration for shadcn pattern
  const chartConfig = {
    score: {
      label: "Score",
      color: "var(--chart-1)",
    },
  } satisfies ChartConfig;

  // Check for existing completed analysis
  const checkExistingAnalysis = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const requests = await listAnalyses(sessionId);
      const completedRequest = requests
        .filter((r) => r.status === "completed")
        .sort((a, b) => {
          const aTime = a.completed_at ? new Date(a.completed_at).getTime() : 0;
          const bTime = b.completed_at ? new Date(b.completed_at).getTime() : 0;
          return bTime - aTime;
        })[0];

      if (completedRequest) {
        const result = await getAnalysisResult(completedRequest.id);
        if (result) {
          setAnalytics(result);
          setLoading(false);
          return true;
        }
      }
      setLoading(false);
      return false;
    } catch (err) {
      console.error("[v0] Failed to check existing analysis:", err);
      setLoading(false);
      return false;
    }
  }, [sessionId]);

  // Poll for analysis completion when analyzing
  useEffect(() => {
    if (!analyzing || !currentRequest) return;

    const requestId = currentRequest.id;
    if (
      currentRequest.status === "completed" ||
      currentRequest.status === "failed"
    ) {
      return;
    }

    const pollInterval = setInterval(async () => {
      try {
        const request = await getAnalysisStatus(requestId);
        if (request.status === "completed") {
          clearInterval(pollInterval);
          const result = await getAnalysisResult(requestId);
          if (result) {
            setAnalytics(result);
            setAnalyzing(false);
            setCurrentRequest(null);
          }
        } else if (request.status === "failed") {
          clearInterval(pollInterval);
          setError(request.error_message || "Analysis failed");
          setAnalyzing(false);
          setCurrentRequest(null);
        }
      } catch (err) {
        console.error("[v0] Failed to poll analysis status:", err);
      }
    }, 2000); // Poll every 2 seconds

    return () => clearInterval(pollInterval);
  }, [analyzing, currentRequest]);

  // Start new analysis
  const startAnalysis = useCallback(async () => {
    setShowConfirmDialog(false);
    setAnalyzing(true);
    setError(null);
    try {
      const request = await analyzeSession(sessionId);
      setCurrentRequest(request);

      if (request.status === "completed") {
        const result = await getAnalysisResult(request.id);
        if (result) {
          setAnalytics(result);
          setAnalyzing(false);
          setCurrentRequest(null);
        }
      } else if (request.status === "running" || request.status === "pending") {
        // Polling will be handled by useEffect
      } else if (request.status === "failed") {
        setError(request.error_message || "Analysis failed");
        setAnalyzing(false);
        setCurrentRequest(null);
      }
    } catch (err) {
      console.error("[v0] Failed to start analysis:", err);
      setError("Failed to start analysis");
      setAnalyzing(false);
      setCurrentRequest(null);
    }
  }, [sessionId]);

  // Initial load: check for existing analysis
  useEffect(() => {
    checkExistingAnalysis().then((hasExisting) => {
      if (!hasExisting) {
        setShowConfirmDialog(true);
      }
    });
  }, [checkExistingAnalysis]);

  if (loading) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-center space-y-4">
          <Loader2 className="w-8 h-8 animate-spin mx-auto text-primary" />
          <p className="text-muted-foreground">Loading analytics...</p>
        </div>
      </div>
    );
  }

  if (analyzing) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-center space-y-4">
          <Loader2 className="w-8 h-8 animate-spin mx-auto text-primary" />
          <p className="text-muted-foreground">Analyzing session...</p>
          {currentRequest && (
            <p className="text-sm text-muted-foreground">
              Request ID: {currentRequest.id.substring(0, 8)}...
            </p>
          )}
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-center space-y-4">
          <AlertCircle className="w-12 h-12 mx-auto text-destructive" />
          <p className="text-muted-foreground">{error}</p>
          <Button onClick={startAnalysis}>Retry</Button>
        </div>
      </div>
    );
  }

  if (!analytics) {
    return (
      <>
        <Dialog open={showConfirmDialog} onOpenChange={setShowConfirmDialog}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Start Analysis</DialogTitle>
              <DialogDescription>
                No existing analysis found for this session. Would you like to
                generate a new analysis? This may take a few moments.
              </DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button
                variant="outline"
                onClick={() => {
                  setShowConfirmDialog(false);
                }}
              >
                Cancel
              </Button>
              <Button onClick={startAnalysis}>Start Analysis</Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
        <div className="flex-1 flex items-center justify-center">
          {!showConfirmDialog && (
            <p className="text-muted-foreground">No analytics available</p>
          )}
        </div>
      </>
    );
  }

  const radarData = analytics.ai_quantitative_output.rubric_scores.map(
    (rubric) => ({
      metric: rubric.rubric_name,
      score: (rubric.score / rubric.max_score) * 4 + 1, // 0~1을 1~5로 매핑
    })
  );

  const qualitativeEntries = analytics.ai_qualitative_output.entries;

  return (
    <div className="flex-1 overflow-y-auto min-h-0">
      <div className="p-8 max-w-7xl mx-auto space-y-8">
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
                  {analytics.ai_quantitative_output.rubric_summary.percentage.toFixed(
                    1
                  )}
                  %
                  <span className="text-muted-foreground ml-1 font-normal">
                    (
                    {
                      analytics.ai_quantitative_output.rubric_summary
                        .total_score
                    }
                    /{analytics.ai_quantitative_output.rubric_summary.max_score}
                    )
                  </span>
                </Badge>
              )}
            </div>
            {analytics.ai_quantitative_output.rubric_summary && (
              <p className="text-sm text-muted-foreground mt-3">
                Evaluated{" "}
                {
                  analytics.ai_quantitative_output.rubric_summary
                    .rubrics_evaluated
                }{" "}
                rubrics (
                {
                  analytics.ai_quantitative_output.rubric_summary
                    .rubrics_version
                }
                )
              </p>
            )}
          </CardHeader>
          <CardContent>
            <ChartContainer
              config={chartConfig}
              className="mx-auto aspect-square max-h-[400px]"
            >
              <RadarChart
                data={radarData}
                margin={{ top: 20, right: 30, bottom: 20, left: 30 }}
              >
                <ChartTooltip
                  cursor={false}
                  content={<ChartTooltipContent />}
                  isAnimationActive={false}
                />
                <PolarAngleAxis dataKey="metric" />
                <PolarGrid stroke="var(--border)" />
                <PolarRadiusAxis
                  angle={90}
                  domain={[1, 5]}
                  tick={false}
                  axisLine={false}
                />
                <Radar
                  dataKey="score"
                  fill="var(--color-score)"
                  fillOpacity={0.6}
                  activeDot={{ r: 4 }}
                />
              </RadarChart>
            </ChartContainer>

            {/* Rubric Score Details */}
            <div className="mt-8 space-y-4">
              {analytics.ai_quantitative_output.rubric_scores.map((rubric) => (
                <div key={rubric.rubric_id} className="space-y-2">
                  <div className="flex items-center justify-between">
                    <span className="text-sm font-medium">
                      {rubric.rubric_name}
                    </span>
                    <Badge variant="secondary">
                      {rubric.score}/{rubric.max_score}
                    </Badge>
                  </div>
                  <Progress
                    value={(rubric.score / rubric.max_score) * 100}
                    className="h-2"
                  />
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
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          <Card>
            <CardHeader className="pb-3">
              <CardTitle className="text-sm font-medium flex items-center gap-2">
                <FileCode className="w-4 h-4 text-primary" />
                Code Changes
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Files Modified</span>
                <span className="font-semibold">
                  {
                    analytics.metric_quantitative_output.file_changes
                      .total_files_modified
                  }
                </span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Files Read</span>
                <span className="font-semibold">
                  {
                    analytics.metric_quantitative_output.file_changes
                      .total_files_read
                  }
                </span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Lines Added</span>
                <span className="font-semibold text-green-500">
                  +
                  {
                    analytics.metric_quantitative_output.file_changes
                      .lines_added
                  }
                </span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Lines Removed</span>
                <span className="font-semibold text-red-500">
                  -
                  {
                    analytics.metric_quantitative_output.file_changes
                      .lines_removed
                  }
                </span>
              </div>
              <div className="flex justify-between text-sm border-t pt-3 mt-3">
                <span className="text-muted-foreground">Net Growth</span>
                <span className="font-semibold">
                  {
                    analytics.metric_quantitative_output.file_changes
                      .net_code_growth
                  }
                </span>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-3">
              <CardTitle className="text-sm font-medium flex items-center gap-2">
                <Clock className="w-4 h-4 text-primary" />
                Time Metrics
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Total Duration</span>
                <span className="font-semibold">
                  {
                    analytics.metric_quantitative_output.time_metrics
                      .total_session_time_minutes
                  }{" "}
                  min
                </span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Peak Hours</span>
                <span className="font-semibold">
                  {analytics.metric_quantitative_output.time_metrics.peak_hours.join(
                    ", "
                  )}
                  h
                </span>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-3">
              <CardTitle className="text-sm font-medium flex items-center gap-2">
                <Zap className="w-4 h-4 text-primary" />
                Token Usage
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
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
              <div className="flex justify-between text-sm border-t pt-3 mt-3">
                <span className="text-muted-foreground">Efficiency</span>
                <span className="font-semibold">
                  {(
                    analytics.metric_quantitative_output.token_metrics
                      .token_efficiency * 100
                  ).toFixed(0)}
                  %
                </span>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-3">
              <CardTitle className="text-sm font-medium flex items-center gap-2">
                <Activity className="w-4 h-4 text-primary" />
                Tool Usage
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Total Ops</span>
                <span className="font-semibold">
                  {
                    analytics.metric_quantitative_output.tool_usage
                      .total_operations
                  }
                </span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Success</span>
                <span className="font-semibold text-green-500">
                  {
                    analytics.metric_quantitative_output.tool_usage
                      .successful_operations
                  }
                </span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Failed</span>
                <span className="font-semibold text-red-500">
                  {
                    analytics.metric_quantitative_output.tool_usage
                      .failed_operations
                  }
                </span>
              </div>
              <div className="flex justify-between text-sm border-t pt-3 mt-3">
                <span className="text-muted-foreground">Avg Exec Time</span>
                <span className="font-semibold">
                  {
                    analytics.metric_quantitative_output.tool_usage
                      .average_execution_time_ms
                  }
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
          <CardContent className="space-y-3">
            {Object.entries(
              analytics.metric_quantitative_output.tool_usage.tool_distribution
            ).map(([tool, count]) => (
              <div
                key={tool}
                className="flex items-center justify-between gap-4"
              >
                <span className="text-sm text-muted-foreground min-w-32">
                  {tool}
                </span>
                <div className="flex-1 flex items-center gap-2">
                  <Progress
                    value={
                      (count /
                        analytics.metric_quantitative_output.tool_usage
                          .total_operations) *
                      100
                    }
                    className="h-2"
                  />
                  <span className="text-sm font-semibold min-w-12 text-right">
                    {count}
                  </span>
                </div>
              </div>
            ))}
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
                    {analytics.ai_qualitative_output.summary.total_entries}{" "}
                    entries •{" "}
                    {
                      analytics.ai_qualitative_output.summary
                        .categories_evaluated
                    }{" "}
                    categories
                  </Badge>
                )}
              </div>
              {analytics.ai_qualitative_output.entries_version && (
                <p className="text-sm text-muted-foreground mt-3">
                  Version: {analytics.ai_qualitative_output.entries_version}
                </p>
              )}
            </CardHeader>
            <CardContent>
              <Tabs
                defaultValue={qualitativeEntries[0]?.key || "entries"}
                className="w-full"
              >
                <TabsList
                  className="grid w-full"
                  style={{
                    gridTemplateColumns: `repeat(${qualitativeEntries.length}, 1fr)`,
                  }}
                >
                  {qualitativeEntries.map((entry) => (
                    <TabsTrigger
                      key={entry.key}
                      value={entry.key}
                      className="capitalize"
                    >
                      {entry.title}
                    </TabsTrigger>
                  ))}
                </TabsList>

                {qualitativeEntries.map((entryOutput) => (
                  <TabsContent
                    key={entryOutput.key}
                    value={entryOutput.key}
                    className="space-y-4 mt-6"
                  >
                    {/* Summary */}
                    {entryOutput.summary && (
                      <div className="p-4 bg-muted/50 rounded-lg border">
                        <p className="text-sm text-muted-foreground italic leading-relaxed">
                          {entryOutput.summary}
                        </p>
                      </div>
                    )}
                    {/* Items */}
                    {entryOutput.items.map((item, idx) => (
                      <Card key={`${entryOutput.key}-item-${idx}`}>
                        <CardContent className="pt-5 pb-5">
                          <div
                            className={`prose prose-sm max-w-none ${
                              isDark ? "prose-invert" : ""
                            }`}
                          >
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
          <CardContent className="grid grid-cols-2 md:grid-cols-4 gap-6">
            <div className="space-y-1">
              <div className="text-sm text-muted-foreground">Generated At</div>
              <div className="font-semibold">
                {new Date(analytics.generated_at).toLocaleString()}
              </div>
            </div>
            <div className="space-y-1">
              <div className="text-sm text-muted-foreground">Model Used</div>
              <div className="font-semibold">
                {analytics.model_used || "N/A"}
              </div>
            </div>
            <div className="space-y-1">
              <div className="text-sm text-muted-foreground">
                Analysis Duration
              </div>
              <div className="font-semibold">
                {analytics.analysis_duration_ms}ms
              </div>
            </div>
            <div className="space-y-1">
              <div className="text-sm text-muted-foreground">Session ID</div>
              <div className="font-mono text-xs">
                {analytics.session_id.substring(0, 8)}...
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
