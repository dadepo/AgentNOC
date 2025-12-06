import { useState, useEffect, useRef } from 'react'
import { useNavigate } from 'react-router-dom'
import AlertsSidebar from './AlertsSidebar'
import Dashboard from './Dashboard'
import AlertDetailView from './AlertDetailView'
import DeleteConfirmDialog from './DeleteConfirmDialog'

function MainView({
  alerts,
  setAlerts,
  selectedAlertId,
  setSelectedAlertId,
  selectedAlertData,
  setSelectedAlertData,
  connected,
  setConnected,
  loading,
  setLoading,
  showDeleteDialog,
  setShowDeleteDialog,
  alertToDelete,
  setAlertToDelete,
  error,
  setError,
  fetchAlerts,
  fetchAlertDetails,
  sendChatMessage,
  deleteAlert,
  handleSelectAlert,
  handleDeleteClick,
  handleDeleteConfirm,
  connectSSE,
  reconnectTimeoutRef,
  eventSourceRef,
}) {
  const navigate = useNavigate()

  // Initial fetch on mount
  useEffect(() => {
    fetchAlerts()
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
      console.log('Fetching alert details for ID:', selectedAlertId)
      fetchAlertDetails(selectedAlertId)
    } else {
      setSelectedAlertData(null)
      setLoading((prev) => ({ ...prev, alertDetails: false }))
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedAlertId])

  return (
    <div className="app">
      <header className="header">
        <h1>AgentNOC</h1>
        <div className="header-right">
          <button
            className="settings-btn"
            onClick={() => navigate('/settings')}
            title="Settings"
          >
            <svg
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <circle cx="12" cy="12" r="3" />
              <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
            </svg>
          </button>
          <div className={`status ${connected ? 'connected' : 'disconnected'}`}>
            {connected ? '● Connected' : '○ Disconnected'}
          </div>
        </div>
      </header>

      {error && (
        <div className="error-banner">
          <span>{error}</span>
          <button onClick={() => setError(null)}>×</button>
        </div>
      )}

      <div className="main-container">
        <AlertsSidebar
          alerts={alerts}
          selectedAlertId={selectedAlertId}
          onSelectAlert={handleSelectAlert}
        />

        <div className="main-content">
          {selectedAlertId ? (
            <AlertDetailView
              alertData={selectedAlertData}
              loading={loading.alertDetails}
              onDelete={handleDeleteClick}
              onSendMessage={sendChatMessage}
              sendingMessage={loading.sendingMessage}
            />
          ) : (
            <Dashboard alertCount={alerts.length} />
          )}
        </div>
      </div>

      <DeleteConfirmDialog
        isOpen={showDeleteDialog}
        alertPrefix={
          selectedAlertData?.alert?.details?.prefix || 'this alert'
        }
        onConfirm={handleDeleteConfirm}
        onCancel={() => {
          setShowDeleteDialog(false)
          setAlertToDelete(null)
        }}
      />
    </div>
  )
}

export default MainView

