import { useState } from 'react'

function ChatInput({ onSend, loading, disabled }) {
  const [message, setMessage] = useState('')

  const handleSubmit = (e) => {
    e.preventDefault()
    if (message.trim() && !loading && !disabled) {
      onSend(message.trim())
      setMessage('')
    }
  }

  const handleKeyDown = (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleSubmit(e)
    }
  }

  return (
    <form className="chat-input-form" onSubmit={handleSubmit}>
      <textarea
        className="chat-input"
        value={message}
        onChange={(e) => setMessage(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder="Ask a question about this alert... (Enter to send, Shift+Enter for newline)"
        disabled={loading || disabled}
        rows={3}
      />
      <button
        type="submit"
        className="chat-send-button"
        disabled={!message.trim() || loading || disabled}
      >
        {loading ? 'Sending...' : 'Send'}
      </button>
    </form>
  )
}

export default ChatInput

