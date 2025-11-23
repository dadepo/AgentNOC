import { useEffect, useRef } from 'react'
import ReactMarkdown from 'react-markdown'

function ChatHistory({ messages }) {
  const messagesEndRef = useRef(null)

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }

  useEffect(() => {
    scrollToBottom()
  }, [messages])

  if (messages.length === 0) {
    return (
      <div className="chat-history-empty">
        <p>No chat messages yet. Ask a question to get started.</p>
      </div>
    )
  }

  return (
    <div className="chat-history">
      {messages.map((message) => (
        <div
          key={message.id}
          className={`chat-message chat-message-${message.role} ${
            message.loading ? 'chat-message-loading' : ''
          }`}
        >
          <div className="chat-message-header">
            <span className="chat-message-role">
              {message.role === 'user' ? 'You' : 'Assistant'}
            </span>
            <span className="chat-message-timestamp">
              {message.loading
                ? 'Thinking...'
                : new Date(message.created_at).toLocaleTimeString()}
            </span>
          </div>
          <div className="chat-message-content">
            {message.loading ? (
              <div className="loading-indicator">
                <span className="loading-dot"></span>
                <span className="loading-dot"></span>
                <span className="loading-dot"></span>
              </div>
            ) : message.role === 'assistant' ? (
              <ReactMarkdown>{message.content}</ReactMarkdown>
            ) : (
              <p>{message.content}</p>
            )}
          </div>
        </div>
      ))}
      <div ref={messagesEndRef} />
    </div>
  )
}

export default ChatHistory

