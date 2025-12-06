import { useState } from 'react'

function AlertDataCard({ alert, expanded = false, onToggle }) {
  const [showRaw, setShowRaw] = useState(false)

  if (!alert) {
    return null
  }

  const details = alert.details || {}

  return (
    <div className="alert-data-card">
      <div className="alert-data-header" onClick={onToggle}>
        <h3>📢 Original Alert</h3>
        <span className="expand-icon">{expanded ? '▼' : '▶'}</span>
      </div>

      {expanded && (
        <div className="alert-data-content">
        <div className="alert-data-section">
          <h4>Alert Information</h4>
          <div className="alert-data-field">
            <strong>Message:</strong> {alert.message || 'N/A'}
          </div>
          <div className="alert-data-field">
            <strong>Description:</strong> {alert.description || 'N/A'}
          </div>
          <div className="alert-data-field">
            <strong>Kind:</strong> {details.kind || 'N/A'}
          </div>
        </div>

        <div className="alert-data-section">
          <h4>Prefix Details</h4>
          <div className="alert-data-field">
            <strong>Prefix:</strong> {details.prefix || 'N/A'}
          </div>
          {details.newprefix && (
            <div className="alert-data-field">
              <strong>New Prefix:</strong> {details.newprefix}
            </div>
          )}
          <div className="alert-data-field">
            <strong>ASN:</strong> {details.asn || 'N/A'}
          </div>
          {details.neworigin && (
            <div className="alert-data-field">
              <strong>New Origin:</strong> {details.neworigin}
            </div>
          )}
        </div>

        <div className="alert-data-section">
          <h4>BGP Details</h4>
          <div className="alert-data-field">
            <strong>Paths:</strong> {details.paths || 'N/A'}
          </div>
          <div className="alert-data-field">
            <strong>Peers:</strong> {details.peers || 'N/A'}
          </div>
          <div className="alert-data-field">
            <strong>Summary:</strong> {details.summary || 'N/A'}
          </div>
        </div>

        <div className="alert-data-section">
          <h4>Timestamps</h4>
          <div className="alert-data-field">
            <strong>Earliest:</strong> {details.earliest || 'N/A'}
          </div>
          <div className="alert-data-field">
            <strong>Latest:</strong> {details.latest || 'N/A'}
          </div>
        </div>

        <div className="alert-data-raw-toggle">
          <button 
            className="toggle-button"
            onClick={() => setShowRaw(!showRaw)}
          >
            {showRaw ? '▼' : '▶'} {showRaw ? 'Hide' : 'Show'} Raw Alert JSON
          </button>
          {showRaw && (
            <pre className="raw-response">{JSON.stringify(alert, null, 2)}</pre>
          )}
        </div>
      </div>
      )}
    </div>
  )
}

export default AlertDataCard

