export interface Session {
  id: string
  provider: string
  project_name: string | null
  message_count: number
  created_at: string
  updated_at: string
}

export interface Message {
  role: string
  content: string
  timestamp: string
}

export interface SessionWithMessages extends Session {
  messages: Message[]
}

export interface SearchResult {
  session_id: string
  role: string
  provider: string
  content: string
  timestamp: string
}
