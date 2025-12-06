import { useState, useEffect } from 'react'

function McpServerForm({ server, onSubmit, onCancel, loading }) {
  const [transportType, setTransportType] = useState('http')
  const [formData, setFormData] = useState({
    name: '',
    description: '',
    url: '',
    command: '',
    args: '',
    env: '',
    enabled: true,
  })
  const [errors, setErrors] = useState({})

  useEffect(() => {
    if (server) {
      // Determine transport type from server data
      const isHttp = server.transport_type === 'http'
      setTransportType(isHttp ? 'http' : 'stdio')
      
      setFormData({
        name: server.name || '',
        description: server.description || '',
        url: server.url || '',
        command: server.command || '',
        args: Array.isArray(server.args) ? server.args.join(', ') : '',
        env: server.env ? Object.entries(server.env).map(([k, v]) => `${k}=${v}`).join('\n') : '',
        enabled: server.enabled !== false,
      })
    } else {
      // Reset form for new server
      setTransportType('http')
      setFormData({
        name: '',
        description: '',
        url: '',
        command: '',
        args: '',
        env: '',
        enabled: true,
      })
    }
    setErrors({})
  }, [server])

  const handleChange = (e) => {
    const { name, value, type, checked } = e.target
    setFormData((prev) => ({
      ...prev,
      [name]: type === 'checkbox' ? checked : value,
    }))
    // Clear error when field is edited
    if (errors[name]) {
      setErrors((prev) => ({ ...prev, [name]: null }))
    }
  }

  const handleTransportChange = (type) => {
    setTransportType(type)
    setErrors({})
  }

  const validate = () => {
    const newErrors = {}
    
    if (!formData.name.trim()) {
      newErrors.name = 'Name is required'
    }
    
    if (transportType === 'http') {
      if (!formData.url.trim()) {
        newErrors.url = 'URL is required'
      } else if (!formData.url.startsWith('http://') && !formData.url.startsWith('https://')) {
        newErrors.url = 'URL must start with http:// or https://'
      }
    } else {
      if (!formData.command.trim()) {
        newErrors.command = 'Command is required'
      }
    }
    
    setErrors(newErrors)
    return Object.keys(newErrors).length === 0
  }

  const parseArgs = (argsStr) => {
    if (!argsStr.trim()) return []
    // Split by comma and trim each argument
    return argsStr.split(',').map((arg) => arg.trim()).filter(Boolean)
  }

  const parseEnv = (envStr) => {
    if (!envStr.trim()) return {}
    const env = {}
    envStr.split('\n').forEach((line) => {
      const trimmed = line.trim()
      if (trimmed && trimmed.includes('=')) {
        const [key, ...valueParts] = trimmed.split('=')
        env[key.trim()] = valueParts.join('=').trim()
      }
    })
    return env
  }

  const handleSubmit = (e) => {
    e.preventDefault()
    
    if (!validate()) return
    
    let payload
    if (transportType === 'http') {
      payload = {
        transport_type: 'http',
        name: formData.name.trim(),
        description: formData.description.trim() || undefined,
        url: formData.url.trim(),
        enabled: formData.enabled,
      }
    } else {
      payload = {
        transport_type: 'stdio',
        name: formData.name.trim(),
        description: formData.description.trim() || undefined,
        command: formData.command.trim(),
        args: parseArgs(formData.args),
        env: parseEnv(formData.env),
        enabled: formData.enabled,
      }
    }
    
    onSubmit(payload)
  }

  return (
    <form className="mcp-server-form" onSubmit={handleSubmit}>
      <h4>{server ? 'Edit Server' : 'Add New Server'}</h4>
      
      <div className="form-group">
        <label>Transport Type</label>
        <div className="transport-selector">
          <button
            type="button"
            className={`transport-btn ${transportType === 'http' ? 'active' : ''}`}
            onClick={() => handleTransportChange('http')}
            disabled={!!server}
          >
            HTTP
          </button>
          <button
            type="button"
            className={`transport-btn ${transportType === 'stdio' ? 'active' : ''}`}
            onClick={() => handleTransportChange('stdio')}
            disabled={!!server}
          >
            Stdio
          </button>
        </div>
        {server && (
          <p className="form-hint">Transport type cannot be changed after creation</p>
        )}
      </div>

      <div className="form-group">
        <label htmlFor="name">Name *</label>
        <input
          type="text"
          id="name"
          name="name"
          value={formData.name}
          onChange={handleChange}
          placeholder="e.g., ripestat"
          className={errors.name ? 'error' : ''}
          disabled={loading}
        />
        {errors.name && <span className="error-message">{errors.name}</span>}
      </div>

      <div className="form-group">
        <label htmlFor="description">Description</label>
        <input
          type="text"
          id="description"
          name="description"
          value={formData.description}
          onChange={handleChange}
          placeholder="e.g., RIPEstat API for BGP data"
          disabled={loading}
        />
      </div>

      {transportType === 'http' ? (
        <div className="form-group">
          <label htmlFor="url">URL *</label>
          <input
            type="text"
            id="url"
            name="url"
            value={formData.url}
            onChange={handleChange}
            placeholder="https://example.com/mcp"
            className={errors.url ? 'error' : ''}
            disabled={loading}
          />
          {errors.url && <span className="error-message">{errors.url}</span>}
        </div>
      ) : (
        <>
          <div className="form-group">
            <label htmlFor="command">Command *</label>
            <input
              type="text"
              id="command"
              name="command"
              value={formData.command}
              onChange={handleChange}
              placeholder="e.g., uvx or /usr/local/bin/mcp-server"
              className={errors.command ? 'error' : ''}
              disabled={loading}
            />
            {errors.command && <span className="error-message">{errors.command}</span>}
          </div>

          <div className="form-group">
            <label htmlFor="args">Arguments</label>
            <input
              type="text"
              id="args"
              name="args"
              value={formData.args}
              onChange={handleChange}
              placeholder="--from, git+https://..., mcp-server"
              disabled={loading}
            />
            <p className="form-hint">Comma-separated list of arguments</p>
          </div>

          <div className="form-group">
            <label htmlFor="env">Environment Variables</label>
            <textarea
              id="env"
              name="env"
              value={formData.env}
              onChange={handleChange}
              placeholder="KEY=value&#10;ANOTHER_KEY=another_value"
              rows={3}
              disabled={loading}
            />
            <p className="form-hint">One KEY=value per line</p>
          </div>
        </>
      )}

      <div className="form-group checkbox-group">
        <label>
          <input
            type="checkbox"
            name="enabled"
            checked={formData.enabled}
            onChange={handleChange}
            disabled={loading}
          />
          <span>Enabled</span>
        </label>
      </div>

      <div className="form-actions">
        <button
          type="button"
          className="form-btn form-btn-cancel"
          onClick={onCancel}
          disabled={loading}
        >
          Cancel
        </button>
        <button
          type="submit"
          className="form-btn form-btn-submit"
          disabled={loading}
        >
          {loading ? 'Saving...' : server ? 'Update Server' : 'Add Server'}
        </button>
      </div>
    </form>
  )
}

export default McpServerForm

