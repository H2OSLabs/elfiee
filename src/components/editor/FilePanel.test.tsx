import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { BrowserRouter } from 'react-router-dom'
import { FilePanel } from './FilePanel'

// Mock子组件
vi.mock('./VfsTree', () => ({
  VfsTree: ({ nodes, activeBlockId, onSelect }: any) => (
    <div data-testid="mock-vfs-tree">
      VfsTree
      {nodes?.map((node: any) => (
        <button
          key={node.path}
          onClick={() => onSelect?.(node)}
          data-testid={`vfs-node-${node.path}`}
          className={activeBlockId === node.blockId ? 'active' : ''}
        >
          {node.name}
        </button>
      ))}
    </div>
  ),
}))

vi.mock('./ImportRepositoryModal', () => ({
  ImportRepositoryModal: ({ open, onOpenChange }: any) =>
    open ? (
      <div data-testid="import-modal">
        <button onClick={() => onOpenChange(false)}>Close</button>
      </div>
    ) : null,
}))

// Mock app-store
const mockStore = {
  currentFileId: 'test-file-id' as string | null,
  selectedBlockId: null as string | null,
  files: new Map(),
  getBlocks: vi.fn(() => []),
  getFileMetadata: vi.fn(() => ({ name: 'Test File' })),
  selectBlock: vi.fn(),
  createBlock: vi.fn(),
  createTaskBlock: vi.fn(),
  renameBlock: vi.fn(),
  deleteBlock: vi.fn(),
  loadBlocks: vi.fn(),
  getOutlineTree: vi.fn(() => []),
  getOutlineRepos: vi.fn(() => []),
  getLinkedRepos: vi.fn(() => []),
  getTaskBlocks: vi.fn(() => []),
  getEvents: vi.fn(() => []),
  getActiveEditor: vi.fn(),
  createEntry: vi.fn(),
  renameEntry: vi.fn(),
  renameEntryWithTypeChange: vi.fn(),
  deleteEntry: vi.fn(),
  importDirectory: vi.fn(),
  checkoutWorkspace: vi.fn(),
}

vi.mock('@/lib/app-store', () => ({
  useAppStore: Object.assign(
    (selector: any) => {
      if (typeof selector === 'function') {
        return selector(mockStore)
      }
      return mockStore
    },
    {
      getState: () => mockStore,
    }
  ),
}))

// Mock sonner toast
vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
  },
}))

// Mock tauri-client
vi.mock('@/lib/tauri-client', () => ({
  TauriClient: {
    directory: {
      createEntry: vi.fn(),
      deleteEntry: vi.fn(),
      renameEntry: vi.fn(),
      importDirectory: vi.fn(),
      checkoutWorkspace: vi.fn(),
    },
    block: {
      createBlock: vi.fn(),
      executeCommand: vi.fn(),
    },
  },
}))

// Mock tauri plugin dialog
vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}))

// Mock UI components
vi.mock('@/components/ui/button', () => ({
  Button: ({
    children,
    onClick,
    ...props
  }: {
    children: React.ReactNode
    onClick?: () => void
  }) => (
    <button onClick={onClick} {...props}>
      {children}
    </button>
  ),
}))

vi.mock('@/components/ui/scroll-area', () => ({
  ScrollArea: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="scroll-area">{children}</div>
  ),
}))

vi.mock('@/components/ui/separator', () => ({
  Separator: () => <div data-testid="separator" />,
}))

vi.mock('@/components/ui/dropdown-menu', () => ({
  DropdownMenu: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="dropdown-menu">{children}</div>
  ),
  DropdownMenuTrigger: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="dropdown-trigger">{children}</div>
  ),
  DropdownMenuContent: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="dropdown-content">{children}</div>
  ),
  DropdownMenuItem: ({
    children,
    onClick,
  }: {
    children: React.ReactNode
    onClick?: () => void
  }) => (
    <button onClick={onClick} data-testid="dropdown-item">
      {children}
    </button>
  ),
}))

// Helper to render with router
const renderWithRouter = (component: React.ReactElement) => {
  return render(<BrowserRouter>{component}</BrowserRouter>)
}

describe('FilePanel', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockStore.currentFileId = 'test-file-id'
    mockStore.getBlocks.mockReturnValue([])
    mockStore.getOutlineTree.mockReturnValue([])
    mockStore.getLinkedRepos.mockReturnValue([])
    mockStore.selectedBlockId = null
  })

  describe('Rendering', () => {
    it('should render outline section', () => {
      renderWithRouter(<FilePanel />)

      expect(screen.getByText('Outline')).toBeInTheDocument()
    })

    it('should render scroll area', () => {
      renderWithRouter(<FilePanel />)

      expect(screen.getByTestId('scroll-area')).toBeInTheDocument()
    })
  })

  describe('Block Display', () => {
    it('should display outline repos from store', () => {
      const mockOutlineRepos = [
        {
          blockId: 'outline1',
          name: 'My Workdir',
          tree: [
            { path: 'doc1.md', name: 'doc1.md', type: 'file', blockId: 'b1' },
          ],
        },
      ]
      mockStore.getOutlineRepos.mockReturnValue(mockOutlineRepos as any)

      renderWithRouter(<FilePanel />)

      expect(screen.getByText('My Workdir')).toBeInTheDocument()
      expect(screen.getByText('doc1.md')).toBeInTheDocument()
    })

    it('should display linked repos from store', () => {
      const mockRepos = [
        {
          blockId: 'repo1',
          name: 'Repo 1',
          tree: [
            { path: 'main.rs', name: 'main.rs', type: 'file', blockId: 'b2' },
          ],
        },
      ]
      mockStore.getLinkedRepos.mockReturnValue(mockRepos as any)

      renderWithRouter(<FilePanel />)

      expect(screen.getByText('Repo 1')).toBeInTheDocument()
      expect(screen.getByText('main.rs')).toBeInTheDocument()
    })
  })

  describe('Block Selection', () => {
    it('should call selectBlock when clicking on a node', async () => {
      const user = userEvent.setup()
      const mockOutlineRepos = [
        {
          blockId: 'outline1',
          name: 'My Workdir',
          tree: [
            { path: 'doc1.md', name: 'doc1.md', type: 'file', blockId: 'b1' },
          ],
        },
      ]
      mockStore.getOutlineRepos.mockReturnValue(mockOutlineRepos as any)

      renderWithRouter(<FilePanel />)

      const blockNode = screen.getByTestId('vfs-node-doc1.md')
      await user.click(blockNode)

      await waitFor(() => {
        expect(mockStore.selectBlock).toHaveBeenCalledWith('b1')
      })
    })
  })

  describe('No File State', () => {
    it('should handle no current file gracefully', () => {
      mockStore.currentFileId = null

      renderWithRouter(<FilePanel />)

      // Component should still render without errors
      expect(screen.getByTestId('scroll-area')).toBeInTheDocument()
    })
  })
})
