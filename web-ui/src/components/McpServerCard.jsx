import { useState } from 'react'

function McpServerCard({ server, onEdit, onDelete, onTest, onToggleEnabled }) {
  const [testing, setTesting] = useState(false)
  const [testResult, setTestResult] = useState(null)

  const isHttp = server.transport_type === 'http'

  const handleTest = async () => {
    setTesting(true)
    setTestResult(null)
    try {
      const result = await onTest()
      setTestResult({ success: true, message: 'Connection successful' })
    } catch (err) {
      setTestResult({ success: false, message: err.message || 'Connection failed' })
    } finally {
      setTesting(false)
    }
  }

  return (
    <div className={`mcp-server-card ${!server.enabled ? 'disabled' : ''}`}>
      <div className="server-card-header">
        <div className="server-card-info">
          <h4 className="server-name">{server.name}</h4>
          <div className="server-badges">
            <span className={`transport-badge ${isHttp ? 'http' : 'stdio'}`}>
              {isHttp ? 'HTTP' : 'Stdio'}
            </span>
            <span className={`status-badge ${server.enabled ? 'enabled' : 'disabled'}`}>
              {server.enabled ? 'Enabled' : 'Disabled'}
            </span>
          </div>
        </div>
        <label className="toggle-switch">
          <input
            type="checkbox"
            checked={server.enabled}
            onChange={onToggleEnabled}
          />
          <span className="toggle-slider"></span>
        </label>
      </div>

      {server.description && (
        <p className="server-description">{server.description}</p>
      )}

      <div className="server-details">
        {isHttp ? (
          <div className="detail-row">
            <span className="detail-label">URL:</span>
            <code className="detail-value">{server.url}</code>
          </div>
        ) : (
          <>
            <div className="detail-row">
              <span className="detail-label">Command:</span>
              <code className="detail-value">{server.command}</code>
            </div>
            {server.args && server.args.length > 0 && (
              <div className="detail-row">
                <span className="detail-label">Args:</span>
                <code className="detail-value">{server.args.join(' ')}</code>
              </div>
            )}
          </>
        )}
      </div>

      {testResult && (
        <div className={`test-result ${testResult.success ? 'success' : 'error'}`}>
          {testResult.success ? '✓' : '✗'} {testResult.message}
        </div>
      )}

      <div className="server-card-actions">
        <button
          className="action-btn test-btn"
          onClick={handleTest}
          disabled={testing}
        >
          {testing ? 'Testing...' : 'Test'}
        </button>
        <button className="action-btn edit-btn" onClick={onEdit}>
          Edit
        </button>
        <button className="action-btn delete-btn" onClick={onDelete}>
          Delete
        </button>
      </div>
    </div>
  )
}

export default McpServerCard

