import { useCallback, useEffect, useState } from 'react'
import { Area, AreaChart, CartesianGrid, XAxis } from 'recharts'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  type ChartConfig,
} from '@/components/ui/chart'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { getSessionActivityHistogram, getUserMessageHistogram } from '@/lib/api'
import type { HistogramResponse, TimeRange } from '@/types'

// Time range presets with auto-selected intervals
const TIME_RANGE_CONFIG: Record<TimeRange, { hours: number; intervalMin: number; label: string }> =
  {
    '6h': { hours: 6, intervalMin: 5, label: 'Last 6 hours' },
    '24h': { hours: 24, intervalMin: 15, label: 'Last 24 hours' },
    '7d': { hours: 168, intervalMin: 60, label: 'Last 7 days' },
    '30d': { hours: 720, intervalMin: 360, label: 'Last 30 days' },
  }

const sessionsChartConfig = {
  sessions: {
    label: 'Active Sessions',
    color: 'var(--chart-1)',
  },
} satisfies ChartConfig

const messagesChartConfig = {
  messages: {
    label: 'User Messages',
    color: 'var(--chart-2)',
  },
} satisfies ChartConfig

export function SessionHistogram() {
  const [timeRange, setTimeRange] = useState<TimeRange>('24h')
  const [loading, setLoading] = useState(false)
  const [sessionData, setSessionData] = useState<HistogramResponse | null>(null)
  const [messageData, setMessageData] = useState<HistogramResponse | null>(null)

  const loadData = useCallback(async () => {
    setLoading(true)
    try {
      const config = TIME_RANGE_CONFIG[timeRange]
      const endTime = new Date()
      const startTime = new Date(endTime.getTime() - config.hours * 60 * 60 * 1000)

      const request = {
        start_time: startTime.toISOString(),
        end_time: endTime.toISOString(),
        interval_minutes: config.intervalMin,
      }

      const [sessions, messages] = await Promise.all([
        getSessionActivityHistogram(request),
        getUserMessageHistogram(request),
      ])

      setSessionData(sessions)
      setMessageData(messages)
    } catch (error) {
      console.error('Failed to load histogram data:', error)
    } finally {
      setLoading(false)
    }
  }, [timeRange])

  useEffect(() => {
    loadData()
  }, [loadData])

  // Prepare separate datasets
  const sessionChartData =
    sessionData?.buckets.map((bucket) => ({
      timestamp: bucket.timestamp,
      sessions: bucket.count,
    })) || []

  const messageChartData =
    messageData?.buckets.map((bucket) => ({
      timestamp: bucket.timestamp,
      messages: bucket.count,
    })) || []
  // console.log("11", sessionChartData)
  // console.log("22", messageChartData)

  // Format timestamp for display (respects user's timezone and locale)
  const formatTimestamp = (timestamp: string) => {
    const date = new Date(timestamp)
    const config = TIME_RANGE_CONFIG[timeRange]

    // For fine intervals (5-15 min), show time only
    if (config.intervalMin <= 15) {
      return date.toLocaleTimeString(undefined, {
        hour: '2-digit',
        minute: '2-digit',
      })
    }
    // For hourly, show date and time
    if (config.intervalMin === 60) {
      return date.toLocaleString(undefined, {
        month: 'short',
        day: 'numeric',
        hour: '2-digit',
      })
    }
    // For 6-hour intervals, show date only
    return date.toLocaleDateString(undefined, {
      month: 'short',
      day: 'numeric',
    })
  }

  return (
    <div className="flex-1 flex flex-col min-h-0 gap-4">
      {/* Header with time range selector */}
      <div className="flex items-center justify-between px-2">
        <div>
          <h3 className="text-lg font-semibold">Activity Timeline</h3>
          <p className="text-sm text-muted-foreground">
            {loading
              ? 'Loading histogram data...'
              : `${sessionData?.total_count || 0} sessions, ${
                  messageData?.total_count || 0
                } messages`}
          </p>
        </div>
        <Select value={timeRange} onValueChange={(value) => setTimeRange(value as TimeRange)}>
          <SelectTrigger className="w-[160px] rounded-lg" aria-label="Select time range">
            <SelectValue placeholder="Select range" />
          </SelectTrigger>
          <SelectContent className="rounded-xl">
            {(['6h', '24h', '7d', '30d'] as TimeRange[]).map((range) => (
              <SelectItem key={range} value={range} className="rounded-lg">
                {TIME_RANGE_CONFIG[range].label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {/* Two separate charts */}
      <div className="flex-1 grid grid-cols-1 gap-4 min-h-0 overflow-y-auto">
        {/* Active Sessions Chart */}
        <Card className="flex flex-col min-h-[300px]">
          <CardHeader className="pb-3">
            <CardTitle className="text-base">Active Sessions</CardTitle>
            <CardDescription>Sessions active during each time interval</CardDescription>
          </CardHeader>
          <CardContent className="flex-1 min-h-0">
            <ChartContainer config={sessionsChartConfig} className="h-full w-full">
              <AreaChart data={sessionChartData}>
                <defs>
                  <linearGradient id="fillSessions" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="var(--color-sessions)" stopOpacity={0.8} />
                    <stop offset="95%" stopColor="var(--color-sessions)" stopOpacity={0.1} />
                  </linearGradient>
                </defs>
                <CartesianGrid vertical={false} />
                <XAxis
                  dataKey="timestamp"
                  tickLine={false}
                  axisLine={false}
                  tickMargin={8}
                  minTickGap={32}
                  tickFormatter={formatTimestamp}
                />
                <ChartTooltip
                  cursor={false}
                  content={
                    <ChartTooltipContent
                      labelFormatter={(value) => {
                        return new Date(value).toLocaleString(undefined, {
                          month: 'short',
                          day: 'numeric',
                          hour: '2-digit',
                          minute: '2-digit',
                        })
                      }}
                      indicator="dot"
                    />
                  }
                />
                <Area
                  dataKey="sessions"
                  type="stepAfter"
                  fill="url(#fillSessions)"
                  stroke="var(--color-sessions)"
                  isAnimationActive={false}
                />
              </AreaChart>
            </ChartContainer>
          </CardContent>
        </Card>

        {/* User Messages Chart */}
        <Card className="flex flex-col min-h-[300px]">
          <CardHeader className="pb-3">
            <CardTitle className="text-base">User Messages</CardTitle>
            <CardDescription>User messages sent during each time interval</CardDescription>
          </CardHeader>
          <CardContent className="flex-1 min-h-0">
            <ChartContainer config={messagesChartConfig} className="h-full w-full">
              <AreaChart data={messageChartData}>
                <defs>
                  <linearGradient id="fillMessages" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="var(--color-messages)" stopOpacity={0.8} />
                    <stop offset="95%" stopColor="var(--color-messages)" stopOpacity={0.1} />
                  </linearGradient>
                </defs>
                <CartesianGrid vertical={false} />
                <XAxis
                  dataKey="timestamp"
                  tickLine={false}
                  axisLine={false}
                  tickMargin={8}
                  minTickGap={32}
                  tickFormatter={formatTimestamp}
                />
                <ChartTooltip
                  cursor={false}
                  content={
                    <ChartTooltipContent
                      labelFormatter={(value) => {
                        return new Date(value).toLocaleString(undefined, {
                          month: 'short',
                          day: 'numeric',
                          hour: '2-digit',
                          minute: '2-digit',
                        })
                      }}
                      indicator="dot"
                    />
                  }
                />
                <Area
                  dataKey="messages"
                  type="stepAfter"
                  fill="url(#fillMessages)"
                  stroke="var(--color-messages)"
                  isAnimationActive={false}
                />
              </AreaChart>
            </ChartContainer>
          </CardContent>
        </Card>
      </div>
    </div>
  )
}
