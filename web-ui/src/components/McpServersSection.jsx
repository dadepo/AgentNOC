import { useState, useEffect } from 'react'
import McpServerCard from './McpServerCard'
import McpServerForm from './McpServerForm'

function McpServersSection({ 
  servers, 
  onRefresh, 
  onCreateServer, 
  onUpdateServer, 
  onDeleteServer, 
  onTestServer 
}) {
  const [showForm, setShowForm] = useState(false)
  const [editingServer, setEditingServer] = useState(null)
  const [loading, setLoading] = useState(false)

  useEffect(() => {
    onRefresh()
  }, [])

  const handleAddClick = () => {
    setEditingServer(null)
    setShowForm(true)
  }

  const handleEditClick = (server) => {
    setEditingServer(server)
    setShowForm(true)
  }

  const handleFormSubmit = async (data) => {
    setLoading(true)
    try {
      if (editingServer) {
        await onUpdateServer(editingServer.id, data)
      } else {
        await onCreateServer(data)
      }
      setShowForm(false)
      setEditingServer(null)
    } finally {
      setLoading(false)
    }
  }

  const handleFormCancel = () => {
    setShowForm(false)
    setEditingServer(null)
  }

  const handleDelete = async (server) => {
    if (confirm(`Are you sure you want to delete "${server.name}"?`)) {
      await onDeleteServer(server.id)
    }
  }

  const handleToggleEnabled = async (server) => {
    await onUpdateServer(server.id, { enabled: !server.enabled })
  }

  return (
    <div className="settings-section-content">
      <div className="section-header">
        <div className="section-header-info">
          <h2>MCP Servers</h2>
          <p>Configure Model Context Protocol servers that provide tools for agent analysis.</p>
        </div>
        {!showForm && (
          <button className="add-server-btn" onClick={handleAddClick}>
            + Add Server
          </button>
        )}
      </div>

      {showForm ? (
        <div className="form-container">
          <McpServerForm
            server={editingServer}
            onSubmit={handleFormSubmit}
            onCancel={handleFormCancel}
            loading={loading}
          />
        </div>
      ) : (
        <div className="servers-grid">
          {servers.length === 0 ? (
            <div className="servers-empty">
              <div className="empty-icon">⚡</div>
              <p>No MCP servers configured</p>
              <p className="servers-empty-hint">
                Add a server to enable AI-powered tools for alert analysis
              </p>
              <button className="add-server-btn-large" onClick={handleAddClick}>
                + Add Your First Server
              </button>
            </div>
          ) : (
            servers.map((server) => (
              <McpServerCard
                key={server.id}
                server={server}
                onEdit={() => handleEditClick(server)}
                onDelete={() => handleDelete(server)}
                onTest={() => onTestServer(server.id)}
                onToggleEnabled={() => handleToggleEnabled(server)}
              />
            ))
          )}
        </div>
      )}
    </div>
  )
}

export default McpServersSection

