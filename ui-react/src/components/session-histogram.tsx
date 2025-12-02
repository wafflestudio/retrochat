import { useCallback, useEffect, useState } from 'react'
import { Area, AreaChart, CartesianGrid, XAxis } from 'recharts'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import type { ChartConfig } from '@/components/ui/chart'
import { ChartContainer, ChartTooltip, ChartTooltipContent } from '@/components/ui/chart'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { getSessionActivityHistogram, getUserMessageHistogram } from '@/lib/api'
import type { HistogramResponse, TimeRange } from '@/types'

// Time range presets
const TIME_RANGE_CONFIG: Record<TimeRange, { hours: number; label: string }> = {
  '6h': { hours: 6, label: 'Last 6 hours' },
  '24h': { hours: 24, label: 'Last 24 hours' },
  '7d': { hours: 168, label: 'Last 7 days' },
  '30d': { hours: 720, label: 'Last 30 days' },
}

// Interval options in minutes
const INTERVAL_OPTIONS = [
  { value: 30, label: '30m' },
  { value: 60, label: '1h' },
  { value: 360, label: '6h' },
  { value: 1440, label: '1d' },
  { value: 10080, label: '1w' },
] as const

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
  const [timeRange, setTimeRange] = useState<TimeRange>('30d')
  const [intervalMinutes, setIntervalMinutes] = useState(1440)
  const [loading, setLoading] = useState(false)
  const [sessionData, setSessionData] = useState<HistogramResponse | null>(null)
  const [messageData, setMessageData] = useState<HistogramResponse | null>(null)

  // Get available intervals based on current time range
  const durationMinutes = TIME_RANGE_CONFIG[timeRange].hours * 60
  const availableIntervals = INTERVAL_OPTIONS.filter((option) => option.value < durationMinutes)

  // Reset interval if it becomes longer than duration
  useEffect(() => {
    if (intervalMinutes >= durationMinutes) {
      // Find the largest available interval that's smaller than duration
      const validInterval = availableIntervals[availableIntervals.length - 1]?.value || 30
      setIntervalMinutes(validInterval)
    }
  }, [durationMinutes, intervalMinutes, availableIntervals])

  const loadData = useCallback(async () => {
    setLoading(true)
    try {
      const config = TIME_RANGE_CONFIG[timeRange]
      const endTime = new Date()
      const startTime = new Date(endTime.getTime() - config.hours * 60 * 60 * 1000)

      const request = {
        start_time: startTime.toISOString(),
        end_time: endTime.toISOString(),
        interval_minutes: intervalMinutes,
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
  }, [timeRange, intervalMinutes])

  useEffect(() => {
    loadData()
  }, [loadData])

  // Prepare separate datasets with centered timestamps
  // Offset timestamp by half interval to center the bar on the x-axis
  const halfIntervalMs = (intervalMinutes * 60 * 1000) / 2

  const sessionChartData =
    sessionData?.buckets.map((bucket) => {
      const originalTime = new Date(bucket.timestamp)
      const centeredTime = new Date(originalTime.getTime() + halfIntervalMs)
      return {
        timestamp: centeredTime.toISOString(),
        sessions: bucket.count,
      }
    }) || []

  const messageChartData =
    messageData?.buckets.map((bucket) => {
      const originalTime = new Date(bucket.timestamp)
      const centeredTime = new Date(originalTime.getTime() + halfIntervalMs)
      return {
        timestamp: centeredTime.toISOString(),
        messages: bucket.count,
      }
    }) || []
  // console.log("11", sessionChartData)
  // console.log("22", messageChartData)

  // Format timestamp for display (respects user's timezone and locale)
  const formatTimestamp = (timestamp: string) => {
    const date = new Date(timestamp)

    // For 30min intervals, show time only
    if (intervalMinutes <= 30) {
      return date.toLocaleTimeString(undefined, {
        hour: '2-digit',
        minute: '2-digit',
      })
    }
    // For 1h and 6h intervals, show date and time
    if (intervalMinutes <= 360) {
      return date.toLocaleString(undefined, {
        month: 'short',
        day: 'numeric',
        hour: '2-digit',
      })
    }
    // For 1d and 1w intervals, show date only
    return date.toLocaleDateString(undefined, {
      month: 'short',
      day: 'numeric',
    })
  }

  return (
    <div className="flex-1 flex flex-col min-h-0 gap-4">
      {/* Header with time range and interval selectors */}
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
        <div className="flex items-center gap-2">
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
          <Select
            value={intervalMinutes.toString()}
            onValueChange={(value) => setIntervalMinutes(Number(value))}
          >
            <SelectTrigger className="w-[140px] rounded-lg" aria-label="Select interval">
              <SelectValue placeholder="Select interval" />
            </SelectTrigger>
            <SelectContent className="rounded-xl">
              {availableIntervals.map((option) => (
                <SelectItem
                  key={option.value}
                  value={option.value.toString()}
                  className="rounded-lg"
                >
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
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
