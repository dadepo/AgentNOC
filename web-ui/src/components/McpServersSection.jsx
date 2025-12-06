import { useState, useEffect } from 'react'
import McpServerCard from './McpServerCard'
import McpServerForm from './McpServerForm'

function McpServersSection({ 
  servers, 
  onRefresh, 
  onCreateServer, 
  onUpdateServer, 
  onDeleteServer, 
  onTestServer,
  onEnableNative
}) {
  const [showForm, setShowForm] = useState(false)
  const [editingServer, setEditingServer] = useState(null)
  const [loading, setLoading] = useState(false)
  const [nativeServers, setNativeServers] = useState([])
  const [nativeEnabled, setNativeEnabled] = useState(false)

  useEffect(() => {
    onRefresh()
    fetchNativeServers()
  }, [])

  const fetchNativeServers = async () => {
    try {
      const response = await fetch('/api/mcps?kind=native')
      if (response.ok) {
        const data = await response.json()
        setNativeServers(data)
        setNativeEnabled(data.length > 0)
      }
    } catch (err) {
      console.error('Error fetching native servers:', err)
    }
  }

  const handleToggleNative = async () => {
    const newEnabled = !nativeEnabled
    setLoading(true)
    try {
      await onEnableNative(newEnabled)
      setNativeEnabled(newEnabled)
      await fetchNativeServers()
      await onRefresh()
    } catch (err) {
      console.error('Error toggling native servers:', err)
    } finally {
      setLoading(false)
    }
  }

  const customServers = servers.filter(s => !s.is_native)

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

      {/* Native MCPs Section */}
      <div className="native-mcps-section">
        <div className="native-mcps-header">
          <div>
            <h3>Native MCP Servers</h3>
            <p>Built-in MCP servers that come with AgentNOC. Enable or disable all at once.</p>
          </div>
          <label className="toggle-switch">
            <input
              type="checkbox"
              checked={nativeEnabled}
              onChange={handleToggleNative}
              disabled={loading}
            />
            <span className="toggle-slider"></span>
          </label>
        </div>
        {nativeEnabled && nativeServers.length > 0 && (
          <div className="servers-grid">
            {nativeServers.map((server) => (
              <McpServerCard
                key={server.id}
                server={server}
                onEdit={null} // Native servers can't be edited
                onDelete={null} // Native servers can't be deleted individually
                onTest={() => onTestServer(server.id)}
                onToggleEnabled={null} // Native servers can't be toggled individually
              />
            ))}
          </div>
        )}
      </div>

      {/* Custom MCPs Section */}
      <div className="custom-mcps-section">
        <div className="custom-mcps-header">
          <h3>Custom MCP Servers</h3>
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
            {customServers.length === 0 ? (
              <div className="servers-empty">
                <div className="empty-icon">⚡</div>
                <p>No custom MCP servers configured</p>
                <p className="servers-empty-hint">
                  Add a server to enable AI-powered tools for alert analysis
                </p>
                <button className="add-server-btn-large" onClick={handleAddClick}>
                  + Add Your First Server
                </button>
              </div>
            ) : (
              customServers.map((server) => (
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
    </div>
  )
}

export default McpServersSection

