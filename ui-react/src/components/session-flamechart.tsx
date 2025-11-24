import { useCallback, useEffect, useState } from 'react'
import Plot from 'react-plotly.js'
import { getSessions } from '@/lib/api'
import type { Session } from '@/types'

interface SessionFlamechartProps {
  provider: string | null
}

interface TimeEvent {
  type: 'START' | 'END'
  timestamp: Date
  sessionId: string
}

interface ConcurrencyPoint {
  time: Date
  count: number
}

interface SessionWithLane extends Session {
  lane: number
  start: Date
  end: Date
}

interface ProjectLaneData {
  sessions: SessionWithLane[]
  numLanes: number
  provider: string
}

const PROVIDER_COLORS: Record<string, string> = {
  'Claude Code': 'rgb(75, 192, 192)',
  'Gemini CLI': 'rgb(255, 159, 64)',
  Codex: 'rgb(153, 102, 255)',
  other: 'rgb(201, 203, 207)',
}

export function SessionFlamechart({ provider }: SessionFlamechartProps) {
  const [loading, setLoading] = useState(true)
  const [plotData, setPlotData] = useState<{
    traces: Plotly.Data[]
    layout: Partial<Plotly.Layout>
  } | null>(null)

  const calculateConcurrency = useCallback((sessions: Session[]) => {
    const timeEvents: TimeEvent[] = []
    const sessionMap = new Map<string, number>()

    for (const session of sessions) {
      const start = new Date(session.created_at)
      const end = new Date(session.updated_at)

      // Add 5-minute padding
      const paddedStart = new Date(start.getTime() - 5 * 60 * 1000)
      const paddedEnd = new Date(end.getTime() + 5 * 60 * 1000)

      timeEvents.push({ type: 'START', timestamp: paddedStart, sessionId: session.id })
      timeEvents.push({ type: 'END', timestamp: paddedEnd, sessionId: session.id })
      sessionMap.set(session.id, session.message_count)
    }

    timeEvents.sort((a, b) => a.timestamp.getTime() - b.timestamp.getTime())

    const activeSessions = new Set<string>()
    const concurrencyTimeline: ConcurrencyPoint[] = []
    const messageTimeline: ConcurrencyPoint[] = []
    let prevCount = 0
    let prevMessages = 0

    for (const event of timeEvents) {
      if (concurrencyTimeline.length > 0) {
        concurrencyTimeline.push({ time: event.timestamp, count: prevCount })
        messageTimeline.push({ time: event.timestamp, count: prevMessages })
      }

      if (event.type === 'START') {
        activeSessions.add(event.sessionId)
      } else {
        activeSessions.delete(event.sessionId)
      }

      const sessionCount = activeSessions.size
      const messageCount = Array.from(activeSessions).reduce(
        (sum, sid) => sum + (sessionMap.get(sid) || 0),
        0
      )

      concurrencyTimeline.push({ time: event.timestamp, count: sessionCount })
      messageTimeline.push({ time: event.timestamp, count: messageCount })
      prevCount = sessionCount
      prevMessages = messageCount
    }

    return { concurrencyTimeline, messageTimeline }
  }, [])

  const assignSessionLanes = useCallback(
    (sessions: SessionWithLane[], maxLanes = 5): SessionWithLane[] => {
      const sorted = [...sessions].sort((a, b) => a.start.getTime() - b.start.getTime())
      const laneEndTimes: (Date | null)[] = Array(maxLanes).fill(null)

      for (const session of sorted) {
        let assigned = false

        for (let laneIdx = 0; laneIdx < maxLanes; laneIdx++) {
          const laneEnd = laneEndTimes[laneIdx]
          if (!laneEnd || laneEnd < session.start) {
            session.lane = laneIdx
            laneEndTimes[laneIdx] = session.end
            assigned = true
            break
          }
        }

        if (!assigned) {
          const earliestLane = laneEndTimes.reduce(
            (minIdx, time, idx) =>
              !time || (laneEndTimes[minIdx] && time < laneEndTimes[minIdx]!) ? idx : minIdx,
            0
          )
          session.lane = earliestLane
          laneEndTimes[earliestLane] = session.end
        }
      }

      return sorted
    },
    []
  )

  const processData = useCallback(
    async (sessions: Session[]) => {
      if (sessions.length === 0) {
        setPlotData(null)
        return
      }

      // Calculate statistics
      const totalMessages = sessions.reduce((sum, s) => sum + s.message_count, 0)
      const totalSessions = sessions.length

      // Group sessions by provider
      const providerSessions = new Map<string, SessionWithLane[]>()

      for (const session of sessions) {
        const sessionWithLane: SessionWithLane = {
          ...session,
          lane: 0,
          start: new Date(session.created_at),
          end: new Date(session.updated_at),
        }

        const providerKey = session.provider || 'other'
        if (!providerSessions.has(providerKey)) {
          providerSessions.set(providerKey, [])
        }
        providerSessions.get(providerKey)!.push(sessionWithLane)
      }

      // Assign lanes for each provider
      const projectLaneData = new Map<string, ProjectLaneData>()

      for (const [providerName, providerSessionList] of providerSessions) {
        const assigned = assignSessionLanes(providerSessionList, 5)
        const numLanes = Math.max(...assigned.map((s) => s.lane)) + 1

        projectLaneData.set(providerName, {
          sessions: assigned,
          numLanes,
          provider: providerName,
        })
      }

      // Build y-axis labels
      const yLabels: string[] = []
      const sortedProviders = Array.from(projectLaneData.keys()).sort()

      for (const providerName of sortedProviders) {
        const data = projectLaneData.get(providerName)!
        for (let lane = 0; lane < data.numLanes; lane++) {
          yLabels.push(`${providerName} L${lane + 1}`)
        }
      }

      // Calculate concurrency
      const { concurrencyTimeline, messageTimeline } = calculateConcurrency(sessions)
      const maxConcurrency = Math.max(...concurrencyTimeline.map((c) => c.count), 0)
      const avgConcurrency =
        concurrencyTimeline.reduce((sum, c) => sum + c.count, 0) / concurrencyTimeline.length || 0
      const maxMessages = Math.max(...messageTimeline.map((c) => c.count), 0)

      // Calculate time range with padding
      const allTimes = sessions.flatMap((s) => [
        new Date(s.created_at).getTime(),
        new Date(s.updated_at).getTime(),
      ])
      const minTimeMs = Math.min(...allTimes)
      const maxTimeMs = Math.max(...allTimes)
      const timeRangeMs = maxTimeMs - minTimeMs
      const padding = timeRangeMs * 0.05 // 5% padding on each side

      const minTime = new Date(minTimeMs - padding)
      const maxTime = new Date(maxTimeMs + padding)

      // Build traces
      const traces: Plotly.Data[] = []

      // Concurrency trace
      const concTimes = concurrencyTimeline
        .filter((c) => c.count > 0 || concurrencyTimeline.some((cc) => cc.count > 0))
        .map((c) => c.time)
      const concCounts = concurrencyTimeline
        .filter((c) => c.count > 0 || concurrencyTimeline.some((cc) => cc.count > 0))
        .map((c) => c.count)

      traces.push({
        x: concTimes,
        y: concCounts,
        type: 'scatter',
        mode: 'lines',
        fill: 'tozeroy',
        line: { color: 'rgb(100, 150, 255)', width: 2 },
        fillcolor: 'rgba(100, 150, 255, 0.3)',
        name: 'Concurrent Sessions',
        hovertemplate: '<b>%{x|%m/%d %H:%M}</b><br>Concurrent: %{y} sessions<extra></extra>',
        xaxis: 'x',
        yaxis: 'y',
      })

      // Concurrent messages trace
      const msgTimes = messageTimeline
        .filter((c) => c.count > 0 || messageTimeline.some((cc) => cc.count > 0))
        .map((c) => c.time)
      const msgCounts = messageTimeline
        .filter((c) => c.count > 0 || messageTimeline.some((cc) => cc.count > 0))
        .map((c) => c.count)

      traces.push({
        x: msgTimes,
        y: msgCounts,
        type: 'scatter',
        mode: 'lines',
        fill: 'tozeroy',
        line: { color: 'rgb(255, 150, 100)', width: 2 },
        fillcolor: 'rgba(255, 150, 100, 0.3)',
        name: 'Concurrent Messages',
        hovertemplate: '<b>%{x|%m/%d %H:%M}</b><br>Messages: %{y}<extra></extra>',
        xaxis: 'x2',
        yaxis: 'y2',
      })

      // Timeline traces
      for (const providerName of sortedProviders) {
        const data = projectLaneData.get(providerName)!
        const color = PROVIDER_COLORS[providerName] || PROVIDER_COLORS.other

        for (const session of data.sessions) {
          const laneLabel = `${providerName} L${session.lane + 1}`
          const durationMs = session.end.getTime() - session.start.getTime()

          traces.push({
            type: 'bar',
            orientation: 'h',
            x: [durationMs],
            y: [laneLabel],
            base: session.start,
            marker: {
              color,
              line: { color: 'white', width: 0.5 },
            },
            hovertemplate: `<b>${providerName}</b><br>L${session.lane + 1}<br>${session.id.substring(0, 8)}<br>${session.message_count} msgs<br>${session.project_name || 'Untitled'}<extra></extra>`,
            showlegend: false,
            xaxis: 'x3',
            yaxis: 'y3',
          })
        }
      }

      // Build background shapes for project grouping
      const shapes: Partial<Plotly.Shape>[] = []

      // Since y-axis is reversed, we need to calculate positions from the end
      const reversedProviders = [...sortedProviders].reverse()

      for (let i = 0; i < reversedProviders.length; i++) {
        const providerName = reversedProviders[i]
        const data = projectLaneData.get(providerName)!
        const numLanes = data.numLanes

        // Add alternating gray backgrounds
        if (i % 2 === 1) {
          // Use provider labels to match the height of active sessions
          const startLabel = `${providerName} L${numLanes}`
          const endLabel = `${providerName} L1`

          shapes.push({
            type: 'rect',
            xref: 'x3',
            yref: 'y3',
            x0: minTime,
            x1: maxTime,
            y0: startLabel,
            y1: endLabel,
            fillcolor: 'rgba(220, 220, 220, 0.4)',
            line: { width: 0 },
            layer: 'below',
          })
        }
      }

      // Add gray boxes between sessions in each lane
      for (const providerName of sortedProviders) {
        const data = projectLaneData.get(providerName)!

        // Group sessions by lane
        const sessionsByLane = new Map<number, SessionWithLane[]>()
        for (const session of data.sessions) {
          if (!sessionsByLane.has(session.lane)) {
            sessionsByLane.set(session.lane, [])
          }
          sessionsByLane.get(session.lane)!.push(session)
        }

        // For each lane, add gray boxes between sessions
        for (const [lane, sessions] of sessionsByLane) {
          const laneLabel = `${providerName} L${lane + 1}`
          const sorted = [...sessions].sort((a, b) => a.start.getTime() - b.start.getTime())

          // Add gray box from timeline start to first session
          if (sorted.length > 0) {
            shapes.push({
              type: 'rect',
              xref: 'x3',
              yref: 'y3',
              x0: minTime,
              x1: sorted[0].start,
              y0: laneLabel,
              y1: laneLabel,
              fillcolor: 'rgba(200, 200, 200, 0.3)',
              line: { width: 0 },
              layer: 'below',
            })
          }

          // Add gray boxes between consecutive sessions
          for (let i = 0; i < sorted.length - 1; i++) {
            const currentSession = sorted[i]
            const nextSession = sorted[i + 1]

            shapes.push({
              type: 'rect',
              xref: 'x3',
              yref: 'y3',
              x0: currentSession.end,
              x1: nextSession.start,
              y0: laneLabel,
              y1: laneLabel,
              fillcolor: 'rgba(200, 200, 200, 0.3)',
              line: { width: 0 },
              layer: 'below',
            })
          }

          // Add gray box from last session to timeline end
          if (sorted.length > 0) {
            shapes.push({
              type: 'rect',
              xref: 'x3',
              yref: 'y3',
              x0: sorted[sorted.length - 1].end,
              x1: maxTime,
              y0: laneLabel,
              y1: laneLabel,
              fillcolor: 'rgba(200, 200, 200, 0.3)',
              line: { width: 0 },
              layer: 'below',
            })
          }
        }
      }

      // Layout
      const layout: Partial<Plotly.Layout> = {
        title: {
          text: `📊 Session Timeline Analysis<br><sub>Showing ${totalSessions} sessions across ${sortedProviders.length} providers</sub>`,
          x: 0.5,
          xanchor: 'center',
        },
        height: Math.max(900, yLabels.length * 30 + 500),
        barmode: 'overlay',
        hovermode: 'closest',
        plot_bgcolor: 'white',
        showlegend: false,
        grid: {
          rows: 3,
          columns: 1,
          roworder: 'top to bottom',
          pattern: 'independent',
        },
        shapes,
        xaxis: {
          title: '',
          type: 'date',
          tickformat: '%H:%M',
          dtick: 3600000,
          gridcolor: 'lightgray',
          range: [minTime, maxTime],
          anchor: 'y',
          domain: [0, 1],
        },
        yaxis: {
          title: 'Concurrent Sessions',
          gridcolor: 'lightgray',
          anchor: 'x',
          domain: [0.88, 0.98],
          range: [0, maxConcurrency * 1.1],
        },
        xaxis2: {
          title: '',
          type: 'date',
          tickformat: '%H:%M',
          dtick: 3600000,
          gridcolor: 'lightgray',
          range: [minTime, maxTime],
          anchor: 'y2',
          domain: [0, 1],
        },
        yaxis2: {
          title: 'Concurrent Messages',
          gridcolor: 'lightgray',
          anchor: 'x2',
          domain: [0.73, 0.83],
          range: [0, maxMessages * 1.1],
        },
        xaxis3: {
          title: 'Time',
          type: 'date',
          tickformat: '%H:%M',
          dtick: 3600000,
          gridcolor: 'lightgray',
          range: [minTime, maxTime],
          anchor: 'y3',
          domain: [0, 1],
        },
        yaxis3: {
          title: 'Projects',
          categoryorder: 'array',
          categoryarray: [...yLabels].reverse(),
          tickfont: { size: 9 },
          anchor: 'x3',
          domain: [0, 0.65],
        },
        annotations: [
          {
            text: `<b>📊 Total Sessions: ${totalSessions}</b>`,
            xref: 'paper',
            yref: 'paper',
            x: 0.15,
            y: 1.02,
            showarrow: false,
            font: { size: 14 },
          },
          {
            text: `<b>💬 Total Messages: ${totalMessages}</b>`,
            xref: 'paper',
            yref: 'paper',
            x: 0.5,
            y: 1.02,
            showarrow: false,
            font: { size: 14 },
          },
          {
            text: `<b>⚙️ Avg Concurrency: x${avgConcurrency.toFixed(1)} (max ${maxConcurrency})</b>`,
            xref: 'paper',
            yref: 'paper',
            x: 0.85,
            y: 1.02,
            showarrow: false,
            font: { size: 14 },
          },
          {
            text: '<b>⚡ Concurrent Sessions</b>',
            xref: 'paper',
            yref: 'paper',
            x: 0.5,
            y: 0.99,
            showarrow: false,
            font: { size: 16 },
            xanchor: 'center',
          },
          {
            text: '<b>💬 Concurrent Messages</b>',
            xref: 'paper',
            yref: 'paper',
            x: 0.5,
            y: 0.84,
            showarrow: false,
            font: { size: 16 },
            xanchor: 'center',
          },
          {
            text: '<b>📅 Project Session Timeline</b>',
            xref: 'paper',
            yref: 'paper',
            x: 0.5,
            y: 0.66,
            showarrow: false,
            font: { size: 16 },
            xanchor: 'center',
          },
        ],
      }

      setPlotData({ traces, layout })
    },
    [calculateConcurrency, assignSessionLanes]
  )

  const loadData = useCallback(async () => {
    setLoading(true)
    try {
      // Fetch a large number of sessions
      const sessions = await getSessions(1, 1000, provider)
      await processData(sessions)
    } catch (error) {
      console.error('Failed to load session data:', error)
    } finally {
      setLoading(false)
    }
  }, [provider, processData])

  useEffect(() => {
    loadData()
  }, [loadData])

  if (loading) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <p className="text-muted-foreground">Loading timeline data...</p>
      </div>
    )
  }

  if (!plotData) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <p className="text-muted-foreground">No data available for visualization</p>
      </div>
    )
  }

  return (
    <div className="flex-1 overflow-auto bg-background p-4">
      <Plot
        data={plotData.traces}
        layout={plotData.layout}
        config={{
          responsive: true,
          displayModeBar: true,
          displaylogo: false,
        }}
        style={{ width: '100%', height: '100%' }}
      />
    </div>
  )
}
