import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Sidebar } from './Sidebar'
import { BrowserRouter } from 'react-router-dom'
import { useAppStore } from '@/lib/app-store'

// Mock dependencies
vi.mock('@/lib/app-store', () => ({
  useAppStore: vi.fn(),
}))

vi.mock('@/lib/tauri-client', () => ({
  TauriClient: {
    file: {
      getSystemEditorId: vi.fn().mockResolvedValue('system-editor-id'),
    },
  },
}))

const mockEditors = [
  { editor_id: 'system-editor-id', name: 'Owner', editor_type: 'Human' },
  { editor_id: 'editor-1', name: 'Alice', editor_type: 'Human' },
  { editor_id: 'editor-2', name: 'Bob', editor_type: 'Human' },
]

describe('Sidebar', () => {
  const setActiveEditorMock = vi.fn()
  const deleteEditorMock = vi.fn()
  const getEditorsMock = vi.fn()
  const getActiveEditorMock = vi.fn()
  const getSystemEditorIdMock = vi.fn().mockResolvedValue('system-editor-id')

  beforeEach(() => {
    vi.clearAllMocks()

    // Mock window.confirm
    vi.stubGlobal('confirm', vi.fn())

    // Default mock implementation (no file open)
    ;(useAppStore as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      currentFileId: null,
      getEditors: getEditorsMock,
      getActiveEditor: getActiveEditorMock,
      setActiveEditor: setActiveEditorMock,
      deleteEditor: deleteEditorMock,
      getSystemEditorId: getSystemEditorIdMock,
      isGlobalCollaborator: vi.fn().mockReturnValue(false),
    })
  })

  const renderSidebar = () => {
    return render(
      <BrowserRouter>
        <Sidebar />
      </BrowserRouter>
    )
  }

  it('renders navigation links', () => {
    renderSidebar()
    const links = screen.getAllByRole('link')
    expect(links).toHaveLength(2)
    expect(links[0]).toHaveAttribute('href', '/')
    expect(links[1]).toHaveAttribute('href', '/editor')
  })

  it('renders user switcher when file is open', async () => {
    const user = userEvent.setup()

    // Setup store for open file (Alice is active)
    ;(useAppStore as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      currentFileId: 'file-123',
      getEditors: () => mockEditors,
      getActiveEditor: () => mockEditors[1], // Alice
      setActiveEditor: setActiveEditorMock,
      deleteEditor: deleteEditorMock,
      getSystemEditorId: getSystemEditorIdMock,
      isGlobalCollaborator: vi.fn().mockReturnValue(false),
    })

    renderSidebar()

    // Find the avatar fallback (AL for Alice)
    const avatar = screen.getByText('AL')
    expect(avatar).toBeInTheDocument()

    // Click to open dropdown
    await user.click(avatar)

    // Check if editors are listed
    expect(screen.getByText('Switch User')).toBeInTheDocument()
    expect(screen.getByText('Owner')).toBeInTheDocument()
    expect(screen.getByText('Alice')).toBeInTheDocument()
    expect(screen.getByText('Bob')).toBeInTheDocument()
  })

  it('calls setActiveEditor when user selected', async () => {
    const user = userEvent.setup()

    // Alice is active
    ;(useAppStore as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      currentFileId: 'file-123',
      getEditors: () => mockEditors,
      getActiveEditor: () => mockEditors[1], // Alice
      setActiveEditor: setActiveEditorMock,
      deleteEditor: deleteEditorMock,
      getSystemEditorId: getSystemEditorIdMock,
      isGlobalCollaborator: vi.fn().mockReturnValue(false),
    })

    renderSidebar()

    // Open dropdown
    await user.click(screen.getByText('AL'))

    // Click Bob
    await user.click(screen.getByText('Bob'))

    expect(setActiveEditorMock).toHaveBeenCalledWith('file-123', 'editor-2')
  })

  it('does not render user switcher when no file is open', () => {
    // Default mock has currentFileId: null
    renderSidebar()

    // Should not find the Avatar
    expect(screen.queryByText('AL')).not.toBeInTheDocument()
    expect(screen.queryByText('Switch User')).not.toBeInTheDocument()
  })

  it('system owner can delete other users', async () => {
    const user = userEvent.setup()

    // Mock confirm to return true
    vi.mocked(window.confirm).mockReturnValue(true)
    deleteEditorMock.mockResolvedValue(undefined)

    // System owner (Owner) is active
    ;(useAppStore as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      currentFileId: 'file-123',
      getEditors: () => mockEditors,
      getActiveEditor: () => mockEditors[0], // System owner
      setActiveEditor: setActiveEditorMock,
      deleteEditor: deleteEditorMock,
      getSystemEditorId: getSystemEditorIdMock,
      isGlobalCollaborator: vi.fn().mockReturnValue(false),
    })

    renderSidebar()

    // Open dropdown
    await user.click(screen.getByText('OW')) // Owner

    // Wait for dropdown to open
    await screen.findByText('Switch User')

    // Find all buttons with title "Delete User"
    const allElements = screen.getByRole('menu').querySelectorAll('button')
    const deleteButtons = Array.from(allElements).filter(
      (btn) => btn.getAttribute('title') === 'Delete User'
    )

    // Should have delete buttons for other users (Alice and Bob)
    expect(deleteButtons.length).toBeGreaterThan(0)

    // Click first delete button
    await user.click(deleteButtons[0])

    // Should show confirmation
    expect(window.confirm).toHaveBeenCalled()

    // Should call deleteEditor
    expect(deleteEditorMock).toHaveBeenCalledWith(
      'file-123',
      expect.any(String)
    )
  })

  it('non-system owner cannot delete users', async () => {
    const user = userEvent.setup()

    // Alice (non-system owner) is active
    ;(useAppStore as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      currentFileId: 'file-123',
      getEditors: () => mockEditors,
      getActiveEditor: () => mockEditors[1], // Alice (not system owner)
      setActiveEditor: setActiveEditorMock,
      deleteEditor: deleteEditorMock,
      getSystemEditorId: getSystemEditorIdMock,
      isGlobalCollaborator: vi.fn().mockReturnValue(false),
    })

    renderSidebar()

    // Open dropdown
    await user.click(screen.getByText('AL')) // Alice

    // Wait for dropdown to open
    await screen.findByText('Switch User')

    // Should not find any delete buttons
    const menu = screen.getByRole('menu')
    const allButtons = menu.querySelectorAll('button')
    const deleteButtons = Array.from(allButtons).filter(
      (btn) => btn.getAttribute('title') === 'Delete User'
    )
    expect(deleteButtons).toHaveLength(0)
  })

  it('does not delete when user cancels confirmation', async () => {
    const user = userEvent.setup()

    // Mock confirm to return false (user cancels)
    vi.mocked(window.confirm).mockReturnValue(false)

    // System owner is active
    ;(useAppStore as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      currentFileId: 'file-123',
      getEditors: () => mockEditors,
      getActiveEditor: () => mockEditors[0], // System owner
      setActiveEditor: setActiveEditorMock,
      deleteEditor: deleteEditorMock,
      getSystemEditorId: getSystemEditorIdMock,
      isGlobalCollaborator: vi.fn().mockReturnValue(false),
    })

    renderSidebar()

    // Open dropdown
    await user.click(screen.getByText('OW'))

    // Wait for dropdown to open
    await screen.findByText('Switch User')

    // Find delete button
    const menu = screen.getByRole('menu')
    const allButtons = menu.querySelectorAll('button')
    const deleteButtons = Array.from(allButtons).filter(
      (btn) => btn.getAttribute('title') === 'Delete User'
    )

    if (deleteButtons.length > 0) {
      await user.click(deleteButtons[0])

      // Should show confirmation
      expect(window.confirm).toHaveBeenCalled()

      // Should NOT call deleteEditor
      expect(deleteEditorMock).not.toHaveBeenCalled()
    }
  })
})
