import { useState } from 'react'
import McpServersSection from './McpServersSection'

const SETTINGS_SECTIONS = [
  { id: 'mcp-servers', label: 'MCP Servers', icon: '⚡' },
  { id: 'agents', label: 'Agents', icon: '🤖', disabled: true },
  { id: 'templates', label: 'Templates', icon: '📋', disabled: true },
  { id: 'preferences', label: 'Preferences', icon: '⚙️', disabled: true },
]

function SettingsPage({ 
  onBack, 
  servers, 
  onRefresh, 
  onCreateServer, 
  onUpdateServer, 
  onDeleteServer, 
  onTestServer,
  onEnableNative
}) {
  const [activeSection, setActiveSection] = useState('mcp-servers')

  const renderContent = () => {
    switch (activeSection) {
      case 'mcp-servers':
        return (
          <McpServersSection
            servers={servers}
            onRefresh={onRefresh}
            onCreateServer={onCreateServer}
            onUpdateServer={onUpdateServer}
            onDeleteServer={onDeleteServer}
            onTestServer={onTestServer}
            onEnableNative={onEnableNative}
          />
        )
      case 'agents':
        return (
          <div className="settings-coming-soon">
            <div className="coming-soon-icon">🤖</div>
            <h3>Agents</h3>
            <p>Configure custom agents with different capabilities and prompts.</p>
            <span className="coming-soon-badge">Coming Soon</span>
          </div>
        )
      case 'templates':
        return (
          <div className="settings-coming-soon">
            <div className="coming-soon-icon">📋</div>
            <h3>Templates</h3>
            <p>Create reusable agent templates for different use cases.</p>
            <span className="coming-soon-badge">Coming Soon</span>
          </div>
        )
      case 'preferences':
        return (
          <div className="settings-coming-soon">
            <div className="coming-soon-icon">⚙️</div>
            <h3>Preferences</h3>
            <p>Customize application behavior and appearance.</p>
            <span className="coming-soon-badge">Coming Soon</span>
          </div>
        )
      default:
        return null
    }
  }

  return (
    <div className="settings-page">
      <header className="settings-page-header">
        <button className="back-button" onClick={onBack}>
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M19 12H5M12 19l-7-7 7-7"/>
          </svg>
          Back
        </button>
        <h1>Settings</h1>
      </header>

      <div className="settings-layout">
        <nav className="settings-sidebar">
          {SETTINGS_SECTIONS.map((section) => (
            <button
              key={section.id}
              className={`settings-nav-item ${activeSection === section.id ? 'active' : ''} ${section.disabled ? 'disabled' : ''}`}
              onClick={() => !section.disabled && setActiveSection(section.id)}
              disabled={section.disabled}
            >
              <span className="nav-icon">{section.icon}</span>
              <span className="nav-label">{section.label}</span>
              {section.disabled && <span className="nav-soon">Soon</span>}
            </button>
          ))}
        </nav>

        <main className="settings-content">
          {renderContent()}
        </main>
      </div>
    </div>
  )
}

export default SettingsPage

