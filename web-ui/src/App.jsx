import { useState, useEffect, useRef } from 'react'
import './index.css'

function App() {
  const [messages, setMessages] = useState([])
  const [connected, setConnected] = useState(false)
  const messagesEndRef = useRef(null)
  const messageIdRef = useRef(0)
  const reconnectTimeoutRef = useRef(null)
  const eventSourceRef = useRef(null)

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }

  useEffect(() => {
    scrollToBottom()
  }, [messages])

  const connectSSE = () => {
    if (eventSourceRef.current) {
      eventSourceRef.current.close()
    }

    const eventSource = new EventSource('/api/messages/stream')
    eventSourceRef.current = eventSource

    eventSource.onopen = () => {
      setConnected(true)
      console.log('Connected to message stream')
    }

    eventSource.onmessage = (event) => {
      const message = event.data
      setMessages((prev) => {
        const newMessage = {
          id: messageIdRef.current++,
          content: message,
          timestamp: Date.now(),
        }
        const newMessages = [...prev, newMessage]
        // Keep only last 100 messages
        return newMessages.slice(-100)
      })
    }

    eventSource.onerror = (error) => {
      console.error('EventSource error:', error)
      setConnected(false)
      eventSource.close()
      
      // Exponential backoff or simple retry
      const timeout = Math.min(5000, (reconnectTimeoutRef.current || 1000) * 1.5)
      console.log(`Attempting to reconnect in ${timeout}ms...`)
      
      clearTimeout(reconnectTimeoutRef.current)
      reconnectTimeoutRef.current = setTimeout(() => {
        connectSSE()
      }, timeout)
    }
  }

  useEffect(() => {
    connectSSE()

    return () => {
      if (eventSourceRef.current) {
        eventSourceRef.current.close()
      }
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current)
      }
    }
  }, [])

  const parseMessage = (content) => {
    try {
      // Try to find JSON content within the message
      // The message format from server is often "Agent result: <JSON>" or just string
      if (content.includes("Agent result: ")) {
        const jsonPart = content.split("Agent result: ")[1]
        // Check if it looks like JSON
        if (jsonPart.trim().startsWith('{') || jsonPart.trim().startsWith('[')) {
             // It might be a markdown block with JSON inside, or just JSON
             return <div className="markdown-content" dangerouslySetInnerHTML={{ __html: jsonPart.replace(/\n/g, '<br/>') }} />
        }
        return <div className="text-content">{jsonPart}</div>
      }
      if (content.includes("Hijack Agent result: ")) {
          const report = content.split("Hijack Agent result: ")[1]
          return <pre className="report-content">{report}</pre>
      }
      return <div className="text-content">{content}</div>
    } catch (e) {
      return <div className="text-content">{content}</div>
    }
  }

  return (
    <div className="app">
      <header className="header">
        <h1>NOC Agent</h1>
        <div className={`status ${connected ? 'connected' : 'disconnected'}`}>
          {connected ? '● Connected' : '○ Disconnected'}
        </div>
      </header>
      <main className="messages-container">
        {messages.length === 0 ? (
          <div className="empty-state">Waiting for agent results...</div>
        ) : (
          <div className="messages-list">
            {messages.map((message) => (
              <div key={message.id} className="message">
                <div className="message-header">
                    <span className="timestamp">{new Date(message.timestamp).toLocaleTimeString()}</span>
                </div>
                <div className="message-content">
                    {parseMessage(message.content)}
                </div>
              </div>
            ))}
            <div ref={messagesEndRef} />
          </div>
        )}
      </main>
    </div>
  )
}

export default App

