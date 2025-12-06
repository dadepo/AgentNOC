import { useState } from 'react'
import IncidentReportCard from './IncidentReportCard'
import ChatHistory from './ChatHistory'
import ChatInput from './ChatInput'
import AlertDataCard from './AlertDataCard'

function AlertDetailView({ alertData, loading, onDelete, onSendMessage, sendingMessage }) {
  const [reportExpanded, setReportExpanded] = useState(true)
  const [alertExpanded, setAlertExpanded] = useState(false)

  if (loading) {
    return (
      <div className="alert-detail-loading">
        <p>Loading alert details...</p>
      </div>
    )
  }

  if (!alertData) {
    return (
      <div className="alert-detail-loading">
        <p>No alert data available. Please select an alert.</p>
      </div>
    )
  }

  const prefix = alertData.alert?.details?.prefix || alertData.alert?.prefix || 'Unknown'
  const timestamp = new Date(alertData.created_at || Date.now()).toLocaleString()

  return (
    <div className="alert-detail-view">
      <div className="alert-detail-header">
        <div className="alert-header-info">
          <h2>{prefix}</h2>
          <span className="alert-timestamp">{timestamp}</span>
        </div>
        <button
          className="delete-button"
          onClick={onDelete}
          disabled={loading}
          title="Delete alert"
        >
          🗑️ Delete
        </button>
      </div>

      <div className="alert-detail-content">
        <AlertDataCard 
          alert={alertData.alert} 
          expanded={alertExpanded}
          onToggle={() => setAlertExpanded(!alertExpanded)}
        />

        {reportExpanded && (
          <IncidentReportCard 
            rawResponse={alertData.initial_response}
          />
        )}

        <div className="chat-section">
          <h3>💬 Chat with Agent</h3>
          <ChatHistory messages={alertData.chat_messages || []} />
          <ChatInput
            onSend={onSendMessage}
            loading={sendingMessage}
            disabled={loading}
          />
        </div>
      </div>
    </div>
  )
}

export default AlertDetailView

