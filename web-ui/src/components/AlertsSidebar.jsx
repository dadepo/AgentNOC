import AlertListItem from './AlertListItem'

function AlertsSidebar({ alerts, selectedAlertId, onSelectAlert }) {
  return (
    <div className="alerts-sidebar">
      <div className="sidebar-header">
        <h2>Alerts</h2>
        <span className="alert-count">{alerts.length}</span>
      </div>
      <div className="alerts-list">
        {alerts.length === 0 ? (
          <div className="empty-state">
            <p>No alerts yet</p>
            <p className="empty-state-hint">
              Alerts will appear here when detected
            </p>
          </div>
        ) : (
          alerts.map((alert) => (
            <AlertListItem
              key={alert.id}
              alert={alert}
              isSelected={alert.id === selectedAlertId}
              onClick={() => onSelectAlert(alert.id)}
            />
          ))
        )}
      </div>
    </div>
  )
}

export default AlertsSidebar

