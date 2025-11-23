import { useState } from 'react'

function IncidentReportCard({ report, rawResponse }) {
  const [showRaw, setShowRaw] = useState(false)

  // Try to parse JSON response
  let parsedReport = null
  let parseError = null
  
  try {
    // Remove markdown code blocks if present
    let cleanJson = rawResponse.trim()
    if (cleanJson.startsWith('```')) {
      // Extract JSON from markdown code block
      const match = cleanJson.match(/```(?:json)?\s*\n?([\s\S]*?)\n?```/)
      if (match) {
        cleanJson = match[1]
      }
    }
    parsedReport = JSON.parse(cleanJson)
  } catch (e) {
    parseError = e.message
  }

  // Fallback to raw text if parsing fails
  if (!parsedReport) {
    return (
      <div className="incident-report-card">
        <div className="report-error">
          <h3>⚠️ Report Parse Error</h3>
          <p>Unable to parse structured report. Showing raw response.</p>
          <details>
            <summary>Error Details</summary>
            <code>{parseError}</code>
          </details>
        </div>
        <div className="report-raw">
          <pre>{rawResponse}</pre>
        </div>
      </div>
    )
  }

  const getSeverityColor = (severity) => {
    const colors = {
      Critical: '#dc2626',
      High: '#ea580c',
      Medium: '#d97706',
      Low: '#65a30d',
      Info: '#0891b2'
    }
    return colors[severity] || '#6b7280'
  }

  return (
    <div className="incident-report-card">
      {/* Header with Severity Badge */}
      <div className="report-header" style={{ borderLeftColor: getSeverityColor(parsedReport.severity) }}>
        <div className="severity-indicator">
          <span 
            className="severity-badge-large" 
            style={{ backgroundColor: getSeverityColor(parsedReport.severity) }}
          >
            {parsedReport.severity} SEVERITY
          </span>
        </div>
        <div className="summary-text">
          {parsedReport.summary}
        </div>
      </div>

      {/* Key Facts Grid */}
      <div className="key-facts-grid">
        <div className="fact-card">
          <div className="fact-label">Affected Prefix</div>
          <div className="fact-value">{parsedReport.key_facts?.affected_prefix || 'N/A'}</div>
        </div>
        <div className="fact-card">
          <div className="fact-label">Observed ASN</div>
          <div className="fact-value">{parsedReport.key_facts?.observed_asn || 'N/A'}</div>
        </div>
        <div className="fact-card">
          <div className="fact-label">Expected ASN</div>
          <div className="fact-value">{parsedReport.key_facts?.expected_asn || 'Unknown'}</div>
        </div>
        <div className="fact-card">
          <div className="fact-label">Duration</div>
          <div className="fact-value">{parsedReport.key_facts?.duration || 'N/A'}</div>
        </div>
        <div className="fact-card">
          <div className="fact-label">Reporting Peers</div>
          <div className="fact-value">{parsedReport.key_facts?.peer_count || 0}</div>
        </div>
      </div>

      {/* Immediate Actions */}
      <div className="actions-section">
        <h3>⚡ Immediate Actions</h3>
        <ol className="actions-list">
          {parsedReport.immediate_actions?.map((action, idx) => (
            <li key={idx}>{action}</li>
          )) || <li>No specific actions recommended</li>}
        </ol>
      </div>

      {/* Risk Assessment */}
      <div className="risk-section">
        <h3>⚠️ Risk Assessment</h3>
        <p>{parsedReport.risk_assessment}</p>
      </div>

      {/* Tool Notes (if any) */}
      {parsedReport.tool_notes && (
        <div className="tool-notes">
          <strong>Note:</strong> {parsedReport.tool_notes}
        </div>
      )}

      {/* Toggle for Raw Response */}
      <div className="raw-toggle">
        <button 
          className="toggle-button"
          onClick={() => setShowRaw(!showRaw)}
        >
          {showRaw ? '▼' : '▶'} {showRaw ? 'Hide' : 'Show'} Raw Response
        </button>
        {showRaw && (
          <pre className="raw-response">{rawResponse}</pre>
        )}
      </div>
    </div>
  )
}

export default IncidentReportCard

