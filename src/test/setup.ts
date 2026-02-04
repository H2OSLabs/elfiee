import { vi, afterEach, beforeEach } from 'vitest'
import { cleanup } from '@testing-library/react'
import '@testing-library/jest-dom'

// Elfiee Test Discipline: A truly reactive mock store
const { mockStoreInstance, notifySubscribers } = vi.hoisted(() => {
  const subscribers = new Set<() => void>()

  const createInitialState = () => ({
    currentFileId: null,
    selectedBlockId: null,
    files: new Map(),
    // Mock actions
    openFile: vi.fn(),
    setCurrentFile: vi.fn(),
    getFileInfo: vi.fn(),
    listOpenFiles: vi.fn(),
    initializeOpenFiles: vi.fn(),
    saveFile: vi.fn(),
    createFile: vi.fn(),
    renameFile: vi.fn(),
    duplicateFile: vi.fn(),
    closeFile: vi.fn(),
    loadBlocks: vi.fn(),
    loadEditors: vi.fn(),
    loadGrants: vi.fn(),
    loadEvents: vi.fn(),
    ensureSystemOutline: vi.fn(),
    selectBlock: vi.fn(),
    updateBlock: vi.fn(),
    createBlock: vi.fn(),
    deleteBlock: vi.fn(),
    renameBlock: vi.fn(),
    createEditor: vi.fn(),
    deleteEditor: vi.fn(),
    setActiveEditor: vi.fn(),
    grantCapability: vi.fn(),
    revokeCapability: vi.fn(),
    updateBlockMetadata: vi.fn(),
    restoreToEvent: vi.fn(),
    checkPermission: vi.fn(),
    createEntry: vi.fn(),
    renameEntry: vi.fn(),
    deleteEntry: vi.fn(),
    importDirectory: vi.fn(),
    checkoutWorkspace: vi.fn(),
    getSystemEditorId: vi.fn().mockResolvedValue('system-editor-default'),
    // Mock getters
    getFileMetadata: vi.fn(),
    getBlocks: vi.fn().mockReturnValue([]),
    getBlock: vi.fn(),
    getActiveEditor: vi.fn(),
    getEditors: vi.fn().mockReturnValue([]),
    getEvents: vi.fn().mockReturnValue([]),
    getGrants: vi.fn().mockReturnValue([]),
    getOutlineTree: vi.fn().mockReturnValue([]),
    getLinkedRepos: vi.fn().mockReturnValue([]),
    // Agent operations
    createAgent: vi.fn(),
    enableAgent: vi.fn(),
    disableAgent: vi.fn(),
    getAgentBlocks: vi.fn().mockReturnValue([]),
    // Global collaborator operations
    addGlobalCollaborator: vi.fn(),
    isGlobalCollaborator: vi.fn().mockReturnValue(false),
    // Block grants
    getBlockGrants: vi.fn().mockReturnValue([]),
  })

  const instance = createInitialState()

  return {
    mockStoreInstance: instance,
    notifySubscribers: () => subscribers.forEach((fn) => fn()),
  }
})

vi.mock('@/lib/app-store', () => {
  const useAppStoreMock: any = vi.fn((selector) => {
    return selector ? selector(mockStoreInstance) : mockStoreInstance
  })

  useAppStoreMock.getState = vi.fn(() => mockStoreInstance)
  useAppStoreMock.setState = vi.fn((update: any) => {
    const nextState =
      typeof update === 'function' ? update(mockStoreInstance) : update
    if (nextState.files && nextState.files !== mockStoreInstance.files) {
      mockStoreInstance.files = new Map(nextState.files)
    }
    Object.assign(mockStoreInstance, nextState)
    notifySubscribers()
  })

  return {
    useAppStore: useAppStoreMock,
  }
})

// Mock @tauri-apps/api/core
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  Channel: vi.fn(),
}))

// Mock @tauri-apps/api/event (used by typed events in bindings.ts)
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
  once: vi.fn(() => Promise.resolve(() => {})),
  emit: vi.fn(),
}))

// Mock @tauri-apps/plugin-dialog
vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
  save: vi.fn(),
}))

// Setup before each test
beforeEach(() => {
  vi.clearAllMocks()
  mockStoreInstance.currentFileId = null
  mockStoreInstance.selectedBlockId = null
  mockStoreInstance.files = new Map()
  mockStoreInstance.getBlocks.mockReturnValue([])
  mockStoreInstance.getEvents.mockReturnValue([])
  mockStoreInstance.getGrants.mockReturnValue([])
  mockStoreInstance.getEditors.mockReturnValue([])
  mockStoreInstance.getOutlineTree.mockReturnValue([])
  mockStoreInstance.getLinkedRepos.mockReturnValue([])
  mockStoreInstance.getFileMetadata.mockReturnValue(null)
})

afterEach(() => {
  cleanup()
})

Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation((query) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
})
