import { useState, useEffect, useRef } from 'react'
import { Routes, Route } from 'react-router-dom'
import './index.css'
import MainView from './components/MainView'
import SettingsPage from './components/SettingsPage'

function App() {
  const [alerts, setAlerts] = useState([])
  const [selectedAlertId, setSelectedAlertId] = useState(null)
  const [selectedAlertData, setSelectedAlertData] = useState(null)
  const [connected, setConnected] = useState(false)
  const [loading, setLoading] = useState({
    alerts: false,
    alertDetails: false,
    sendingMessage: false,
  })
  const [showDeleteDialog, setShowDeleteDialog] = useState(false)
  const [alertToDelete, setAlertToDelete] = useState(null)
  const [error, setError] = useState(null)
  
  // MCP Servers state
  const [mcpServers, setMcpServers] = useState([])

  const reconnectTimeoutRef = useRef(null)
  const eventSourceRef = useRef(null)

  // Fetch alerts list
  const fetchAlerts = async () => {
    setLoading((prev) => ({ ...prev, alerts: true }))
    setError(null)
    try {
      const response = await fetch('/api/alerts')
      if (!response.ok) {
        throw new Error('Failed to fetch alerts')
      }
      const data = await response.json()
      setAlerts(data)
    } catch (err) {
      console.error('Error fetching alerts:', err)
      setError('Failed to load alerts. Please refresh the page.')
    } finally {
      setLoading((prev) => ({ ...prev, alerts: false }))
    }
  }

  // Fetch alert details
  const fetchAlertDetails = async (id) => {
    if (!id) {
      console.log('fetchAlertDetails called with no ID')
      return
    }
    
    console.log('Starting fetchAlertDetails for ID:', id)
    setLoading((prev) => ({ ...prev, alertDetails: true }))
    setError(null)
    try {
      console.log('Fetching from:', `/api/alerts/${id}`)
      const response = await fetch(`/api/alerts/${id}`)
      console.log('Response status:', response.status, response.ok)
      
      if (!response.ok) {
        if (response.status === 404) {
          console.log('Alert not found (404)')
          setSelectedAlertId(null)
          setSelectedAlertData(null)
          await fetchAlerts() // Refresh alerts list
          setLoading((prev) => ({ ...prev, alertDetails: false }))
          return
        }
        const errorText = await response.text()
        console.error('Response error:', errorText)
        throw new Error(`Failed to fetch alert details: ${response.status}`)
      }
      const data = await response.json()
      console.log('Fetched alert data:', data) // Debug log
      // Accept any data structure as long as it's not null/undefined
      if (data) {
        console.log('Setting alert data')
        setSelectedAlertData(data)
      } else {
        console.error('Invalid data structure:', data)
        throw new Error('Invalid response format from server')
      }
    } catch (err) {
      console.error('Error fetching alert details:', err)
      setError(`Failed to load alert details: ${err.message}`)
      setSelectedAlertData(null)
    } finally {
      console.log('Clearing loading state')
      setLoading((prev) => ({ ...prev, alertDetails: false }))
    }
  }

  // Send chat message
  const sendChatMessage = async (message) => {
    if (!selectedAlertId || !selectedAlertData) return

    setLoading((prev) => ({ ...prev, sendingMessage: true }))
    setError(null)

    // Optimistically add user message immediately
    const tempUserMessage = {
      id: `temp-${Date.now()}`,
      alert_id: selectedAlertId,
      role: 'user',
      content: message,
      created_at: new Date().toISOString(),
    }

    // Add temporary loading message for assistant response
    const tempLoadingMessage = {
      id: `loading-${Date.now()}`,
      alert_id: selectedAlertId,
      role: 'assistant',
      content: '',
      created_at: new Date().toISOString(),
      loading: true,
    }

    // Update UI immediately with user message and loading indicator
    setSelectedAlertData((prev) => ({
      ...prev,
      chat_messages: [
        ...(prev.chat_messages || []),
        tempUserMessage,
        tempLoadingMessage,
      ],
    }))

    try {
      const response = await fetch(`/api/alerts/${selectedAlertId}/chat`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ message }),
      })

      if (!response.ok) {
        throw new Error('Failed to send message')
      }

      const result = await response.json()

      // Replace loading message with actual response
      setSelectedAlertData((prev) => {
        const messages = prev.chat_messages || []
        // Remove the loading message
        const withoutLoading = messages.filter((msg) => !msg.loading)
        // Add the actual assistant response
        const assistantMessage = {
          id: result.message_id,
          alert_id: selectedAlertId,
          role: 'assistant',
          content: result.response,
          created_at: new Date().toISOString(),
        }
        return {
          ...prev,
          chat_messages: [...withoutLoading, assistantMessage],
        }
      })
    } catch (err) {
      console.error('Error sending message:', err)
      setError('Failed to send message. Please try again.')

      // Remove the optimistic messages on error
      setSelectedAlertData((prev) => {
        const messages = prev.chat_messages || []
        return {
          ...prev,
          chat_messages: messages.filter(
            (msg) => !msg.id.startsWith('temp-') && !msg.id.startsWith('loading-')
          ),
        }
      })
    } finally {
      setLoading((prev) => ({ ...prev, sendingMessage: false }))
    }
  }

  // Delete alert
  const deleteAlert = async (id) => {
    setError(null)
    try {
      const response = await fetch(`/api/alerts/${id}`, {
        method: 'DELETE',
      })

      if (!response.ok) {
        throw new Error('Failed to delete alert')
      }

      // Remove from alerts list
      setAlerts((prev) => prev.filter((alert) => alert.id !== id))

      // Clear selection if deleted alert was selected
      if (selectedAlertId === id) {
        setSelectedAlertId(null)
        setSelectedAlertData(null)
      }

      setShowDeleteDialog(false)
      setAlertToDelete(null)
    } catch (err) {
      console.error('Error deleting alert:', err)
      setError('Failed to delete alert. Please try again.')
      setShowDeleteDialog(false)
      setAlertToDelete(null)
    }
  }

  // Handle alert selection
  const handleSelectAlert = (id) => {
    setSelectedAlertId(id)
    fetchAlertDetails(id)
  }

  // Handle delete button click
  const handleDeleteClick = () => {
    if (selectedAlertData?.alert?.details?.prefix) {
      setAlertToDelete(selectedAlertId)
      setShowDeleteDialog(true)
    }
  }

  // Handle delete confirmation
  const handleDeleteConfirm = () => {
    if (alertToDelete) {
      deleteAlert(alertToDelete)
    }
  }

  // =====================
  // MCP Server API Functions
  // =====================
  
  const fetchMcpServers = async () => {
    try {
      const response = await fetch('/api/mcps')
      if (!response.ok) {
        throw new Error('Failed to fetch MCP servers')
      }
      const data = await response.json()
      setMcpServers(data)
    } catch (err) {
      console.error('Error fetching MCP servers:', err)
      setError('Failed to load MCP servers.')
    }
  }

  const createMcpServer = async (serverData) => {
    try {
      const response = await fetch('/api/mcps', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(serverData),
      })
      
      if (!response.ok) {
        const errorData = await response.text()
        throw new Error(errorData || 'Failed to create server')
      }
      
      const newServer = await response.json()
      setMcpServers((prev) => [...prev, newServer])
      return newServer
    } catch (err) {
      console.error('Error creating MCP server:', err)
      setError(`Failed to create server: ${err.message}`)
      throw err
    }
  }

  const updateMcpServer = async (id, serverData) => {
    try {
      const response = await fetch(`/api/mcps/${id}`, {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(serverData),
      })
      
      if (!response.ok) {
        const errorData = await response.text()
        throw new Error(errorData || 'Failed to update server')
      }
      
      const updatedServer = await response.json()
      setMcpServers((prev) =>
        prev.map((s) => (s.id === id ? updatedServer : s))
      )
      return updatedServer
    } catch (err) {
      console.error('Error updating MCP server:', err)
      setError(`Failed to update server: ${err.message}`)
      throw err
    }
  }

  const deleteMcpServer = async (id) => {
    try {
      const response = await fetch(`/api/mcps/${id}`, {
        method: 'DELETE',
      })
      
      if (!response.ok) {
        throw new Error('Failed to delete server')
      }
      
      setMcpServers((prev) => prev.filter((s) => s.id !== id))
    } catch (err) {
      console.error('Error deleting MCP server:', err)
      setError('Failed to delete server.')
      throw err
    }
  }

  const testMcpServer = async (id) => {
    const response = await fetch(`/api/mcps/${id}/test`, {
      method: 'POST',
    })
    
    if (!response.ok) {
      const errorData = await response.text()
      throw new Error(errorData || 'Connection test failed')
    }
    
    return await response.json()
  }

  const enableNativeMcpServers = async (enabled) => {
    try {
      const response = await fetch('/api/mcps/enable-native', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ enabled }),
      })
      
      if (!response.ok) {
        const errorData = await response.text()
        throw new Error(errorData || 'Failed to enable/disable native MCP servers')
      }
      
      // Refresh the server list
      await fetchMcpServers()
    } catch (err) {
      console.error('Error enabling/disabling native MCP servers:', err)
      setError(`Failed to enable/disable native servers: ${err.message}`)
      throw err
    }
  }

  // SSE connection
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
      try {
        const data = JSON.parse(event.data)
        handleSSEEvent(data)
      } catch (err) {
        // If not JSON, ignore (might be keep-alive or other non-JSON message)
        console.log('Non-JSON SSE message:', event.data)
      }
    }

    eventSource.onerror = (error) => {
      console.error('EventSource error:', error)
      setConnected(false)
      eventSource.close()
      
      // Exponential backoff retry
      const timeout = Math.min(
        5000,
        (reconnectTimeoutRef.current || 1000) * 1.5
      )
      console.log(`Attempting to reconnect in ${timeout}ms...`)
      
      clearTimeout(reconnectTimeoutRef.current)
      reconnectTimeoutRef.current = setTimeout(() => {
        connectSSE()
      }, timeout)
    }
  }

  // Handle SSE events
  const handleSSEEvent = (event) => {
    switch (event.type) {
      case 'new_alert':
        // Refresh alerts list when new alert arrives
        fetchAlerts()
        break

      case 'chat_message':
        // If the selected alert matches, refresh its details
        if (event.alert_id === selectedAlertId) {
          fetchAlertDetails(selectedAlertId)
        }
        break

      case 'alert_deleted':
        // Remove from alerts list
        setAlerts((prev) => prev.filter((alert) => alert.id !== event.alert_id))

        // Clear selection if deleted alert was selected
        if (selectedAlertId === event.alert_id) {
          setSelectedAlertId(null)
          setSelectedAlertData(null)
        }
        break

      case 'error':
        console.error('SSE error:', event.message)
        setError(event.message)
        break

      case 'health_check':
        // Optional: could show health status
        break

      default:
        console.log('Unknown SSE event type:', event.type)
    }
  }

  // Initial fetch on mount
  useEffect(() => {
    fetchAlerts()
    fetchMcpServers()
  }, [])

  // Connect to SSE on mount
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

  // Fetch alert details when selection changes
  useEffect(() => {
    if (selectedAlertId) {
      console.log('Fetching alert details for ID:', selectedAlertId) // Debug log
      fetchAlertDetails(selectedAlertId)
    } else {
      setSelectedAlertData(null)
      setLoading((prev) => ({ ...prev, alertDetails: false }))
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedAlertId])

  return (
    <Routes>
      <Route
        path="/"
        element={
          <MainView
            alerts={alerts}
            setAlerts={setAlerts}
            selectedAlertId={selectedAlertId}
            setSelectedAlertId={setSelectedAlertId}
            selectedAlertData={selectedAlertData}
            setSelectedAlertData={setSelectedAlertData}
            connected={connected}
            setConnected={setConnected}
            loading={loading}
            setLoading={setLoading}
            showDeleteDialog={showDeleteDialog}
            setShowDeleteDialog={setShowDeleteDialog}
            alertToDelete={alertToDelete}
            setAlertToDelete={setAlertToDelete}
            error={error}
            setError={setError}
            fetchAlerts={fetchAlerts}
            fetchAlertDetails={fetchAlertDetails}
            sendChatMessage={sendChatMessage}
            deleteAlert={deleteAlert}
            handleSelectAlert={handleSelectAlert}
            handleDeleteClick={handleDeleteClick}
            handleDeleteConfirm={handleDeleteConfirm}
            connectSSE={connectSSE}
            reconnectTimeoutRef={reconnectTimeoutRef}
            eventSourceRef={eventSourceRef}
          />
        }
      />
      <Route
        path="/settings"
        element={
          <SettingsPage
            servers={mcpServers}
            onRefresh={fetchMcpServers}
            onCreateServer={createMcpServer}
            onUpdateServer={updateMcpServer}
            onDeleteServer={deleteMcpServer}
            onTestServer={testMcpServer}
            onEnableNative={enableNativeMcpServers}
          />
        }
      />
    </Routes>
  )
}

export default App
