function Dashboard({ alertCount }) {
  return (
    <div className="dashboard">
      <div className="dashboard-content">
        <h2>AgentNOC Dashboard</h2>
        <div className="dashboard-section">
          <h3>System Overview</h3>
          <p>
            The AgentNOC monitors BGP alerts and provides automated analysis
            of potential security incidents and routing anomalies.
          </p>
          <div className="dashboard-stats">
            <div className="stat-card">
              <div className="stat-value">{alertCount}</div>
              <div className="stat-label">Total Alerts</div>
            </div>
          </div>
        </div>
        <div className="dashboard-section">
          <h3>Getting Started</h3>
          <div className="instructions">
            <p>
              <strong>Select an alert</strong> from the sidebar to view:
            </p>
            <ul>
              <li>Initial analysis report</li>
              <li>Chat with the agent for follow-up questions</li>
              <li>Detailed alert information</li>
            </ul>
            <p>
              You can ask questions about any alert to get more information or
              clarification.
            </p>
          </div>
        </div>
        <div className="dashboard-section">
          <h3>Features</h3>
          <ul className="features-list">
            <li>Real-time alert monitoring</li>
            <li>Automated incident analysis</li>
            <li>Interactive chat with AI agent</li>
            <li>Persistent alert history</li>
          </ul>
        </div>
      </div>
    </div>
  )
}

export default Dashboard

