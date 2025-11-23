import { useEffect } from 'react'

function DeleteConfirmDialog({ isOpen, alertPrefix, onConfirm, onCancel }) {
  useEffect(() => {
    const handleEscape = (e) => {
      if (e.key === 'Escape' && isOpen) {
        onCancel()
      }
    }
    document.addEventListener('keydown', handleEscape)
    return () => document.removeEventListener('keydown', handleEscape)
  }, [isOpen, onCancel])

  if (!isOpen) return null

  return (
    <div className="dialog-overlay" onClick={onCancel}>
      <div className="dialog-content" onClick={(e) => e.stopPropagation()}>
        <h3>Delete Alert</h3>
        <p>
          Are you sure you want to delete the alert for{' '}
          <strong>{alertPrefix}</strong>?
        </p>
        <p className="dialog-warning">
          This action cannot be undone. All chat history will be deleted.
        </p>
        <div className="dialog-actions">
          <button className="dialog-button dialog-button-cancel" onClick={onCancel}>
            Cancel
          </button>
          <button
            className="dialog-button dialog-button-delete"
            onClick={onConfirm}
          >
            Delete
          </button>
        </div>
      </div>
    </div>
  )
}

export default DeleteConfirmDialog

