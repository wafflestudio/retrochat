import { Activity, BarChart3, Database, FileText, MessageSquare, TrendingUp } from 'lucide-react'
import { useCallback, useEffect, useState } from 'react'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Progress } from '@/components/ui/progress'
import { getSessions } from '@/lib/api'
import { SessionFlamechart } from './session-flamechart'

interface SessionStatisticsProps {
  provider: string | null
}

interface ProviderStats {
  name: string
  count: number
  messageCount: number
}

export function SessionStatistics({ provider }: SessionStatisticsProps) {
  const [view, setView] = useState<'overview' | 'timeline'>('timeline')
  const [loading, setLoading] = useState(true)
  const [stats, setStats] = useState<{
    totalSessions: number
    totalMessages: number
    avgMessagesPerSession: number
    providerBreakdown: ProviderStats[]
    recentActivity: string | null
  }>({
    totalSessions: 0,
    totalMessages: 0,
    avgMessagesPerSession: 0,
    providerBreakdown: [],
    recentActivity: null,
  })

  const loadStatistics = useCallback(async () => {
    setLoading(true)
    try {
      // Fetch a large number of sessions to calculate stats
      // Using a high page size to get all sessions at once
      const allSessions = await getSessions(1, 1000, provider)

      const totalSessions = allSessions.length
      const totalMessages = allSessions.reduce((sum, session) => sum + session.message_count, 0)
      const avgMessagesPerSession =
        totalSessions > 0 ? Math.round(totalMessages / totalSessions) : 0

      // Calculate provider breakdown
      const providerMap = new Map<string, { count: number; messageCount: number }>()
      for (const session of allSessions) {
        const existing = providerMap.get(session.provider) || { count: 0, messageCount: 0 }
        providerMap.set(session.provider, {
          count: existing.count + 1,
          messageCount: existing.messageCount + session.message_count,
        })
      }

      const providerBreakdown: ProviderStats[] = Array.from(providerMap.entries())
        .map(([name, data]) => ({
          name,
          count: data.count,
          messageCount: data.messageCount,
        }))
        .sort((a, b) => b.count - a.count)

      // Get most recent activity
      const recentActivity =
        allSessions.length > 0 ? new Date(allSessions[0].created_at).toLocaleDateString() : null

      setStats({
        totalSessions,
        totalMessages,
        avgMessagesPerSession,
        providerBreakdown,
        recentActivity,
      })
    } catch (error) {
      console.error('Failed to load statistics:', error)
    } finally {
      setLoading(false)
    }
  }, [provider])

  useEffect(() => {
    loadStatistics()
  }, [loadStatistics])

  if (view === 'timeline') {
    return (
      <div className="flex-1 flex flex-col min-h-0 bg-background">
        <div className="p-4 border-b border-border flex items-center justify-between">
          <div>
            <h2 className="text-xl font-bold">Session Timeline</h2>
            <p className="text-sm text-muted-foreground">
              Flamechart view{provider ? ` for ${provider}` : ' across all providers'}
            </p>
          </div>
          <Button variant="outline" onClick={() => setView('overview')} className="gap-2">
            <BarChart3 className="w-4 h-4" />
            Overview
          </Button>
        </div>
        {/* <SessionFlamechart provider={provider} /> */}
      </div>
    )
  }

  if (loading) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <p className="text-muted-foreground">Loading statistics...</p>
      </div>
    )
  }

  return (
    <div className="flex-1 overflow-y-auto min-h-0 bg-background">
      <div className="p-8 max-w-5xl mx-auto space-y-6">
        {/* Header */}
        <div className="flex items-center justify-between">
          <div className="space-y-2">
            <h2 className="text-3xl font-bold tracking-tight">Session Statistics</h2>
            <p className="text-muted-foreground">
              Overview of your chat sessions
              {provider ? ` from ${provider}` : ' across all providers'}
            </p>
          </div>
          <Button variant="outline" onClick={() => setView('timeline')} className="gap-2">
            <Activity className="w-4 h-4" />
            Timeline View
          </Button>
        </div>

        {/* Main Stats Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          <Card>
            <CardHeader className="pb-3">
              <CardTitle className="text-sm font-medium flex items-center gap-2 text-muted-foreground">
                <Database className="w-4 h-4" />
                Total Sessions
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold">{stats.totalSessions.toLocaleString()}</div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-3">
              <CardTitle className="text-sm font-medium flex items-center gap-2 text-muted-foreground">
                <MessageSquare className="w-4 h-4" />
                Total Messages
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold">{stats.totalMessages.toLocaleString()}</div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-3">
              <CardTitle className="text-sm font-medium flex items-center gap-2 text-muted-foreground">
                <TrendingUp className="w-4 h-4" />
                Avg Messages/Session
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold">{stats.avgMessagesPerSession}</div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-3">
              <CardTitle className="text-sm font-medium flex items-center gap-2 text-muted-foreground">
                <FileText className="w-4 h-4" />
                Recent Activity
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">{stats.recentActivity || 'No sessions'}</div>
            </CardContent>
          </Card>
        </div>

        {/* Provider Breakdown */}
        {stats.providerBreakdown.length > 0 && (
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <BarChart3 className="w-5 h-5" />
                Provider Distribution
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              {stats.providerBreakdown.map((providerStat) => (
                <div key={providerStat.name} className="space-y-2">
                  <div className="flex items-center justify-between">
                    <div className="flex flex-col">
                      <span className="text-sm font-medium">{providerStat.name}</span>
                      <span className="text-xs text-muted-foreground">
                        {providerStat.messageCount.toLocaleString()} messages
                      </span>
                    </div>
                    <div className="text-right">
                      <div className="text-sm font-semibold">{providerStat.count} sessions</div>
                      <div className="text-xs text-muted-foreground">
                        {stats.totalSessions > 0
                          ? `${Math.round((providerStat.count / stats.totalSessions) * 100)}%`
                          : '0%'}
                      </div>
                    </div>
                  </div>
                  <Progress
                    value={
                      stats.totalSessions > 0 ? (providerStat.count / stats.totalSessions) * 100 : 0
                    }
                    className="h-2"
                  />
                </div>
              ))}
            </CardContent>
          </Card>
        )}

        {/* Empty State */}
        {stats.totalSessions === 0 && (
          <Card>
            <CardContent className="py-12 text-center">
              <Database className="w-12 h-12 mx-auto mb-4 opacity-20" />
              <p className="text-lg font-medium text-muted-foreground">No sessions found</p>
              <p className="text-sm text-muted-foreground mt-2">
                Import your chat sessions to see statistics
              </p>
            </CardContent>
          </Card>
        )}
      </div>
    </div>
  )
}
