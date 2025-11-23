function AlertListItem({ alert, isSelected, onClick }) {
  const getSeverityColor = (kind) => {
    switch (kind?.toLowerCase()) {
      case 'hijack':
        return '#ef4444' // red
      case 'route_change':
        return '#eab308' // yellow
      case 'configuration':
        return '#3b82f6' // blue
      default:
        return '#6b7280' // gray
    }
  }

  const formatTimestamp = (timestamp) => {
    const date = new Date(timestamp)
    const now = new Date()
    const diffMs = now - date
    const diffMins = Math.floor(diffMs / 60000)
    const diffHours = Math.floor(diffMs / 3600000)
    const diffDays = Math.floor(diffMs / 86400000)

    if (diffMins < 1) return 'Just now'
    if (diffMins < 60) return `${diffMins}m ago`
    if (diffHours < 24) return `${diffHours}h ago`
    if (diffDays < 7) return `${diffDays}d ago`
    return date.toLocaleDateString()
  }

  const prefix = alert.alert_data?.details?.prefix || 'Unknown'
  const kind = alert.alert_data?.details?.kind || 'unknown'
  const severityColor = getSeverityColor(kind)

  return (
    <div
      className={`alert-list-item ${isSelected ? 'selected' : ''}`}
      onClick={onClick}
    >
      <div className="alert-item-header">
        <span className="alert-prefix">{prefix}</span>
        <span
          className="severity-badge"
          style={{ backgroundColor: severityColor }}
        >
          {kind}
        </span>
      </div>
      <div className="alert-item-timestamp">
        {formatTimestamp(alert.created_at)}
      </div>
    </div>
  )
}

export default AlertListItem

