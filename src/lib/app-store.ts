/**
 * App Store - Zustand state management for Elfiee
 *
 * This store manages global application state for editor functionality.
 */

import { create } from 'zustand'
import { TauriClient } from './tauri-client'
import type {
  Editor,
  Block,
  FileMetadata,
  Event,
  Grant,
  AgentCreateV2Payload,
  AgentCreateResult,
  AgentEnableResult,
  AgentDisableResult,
} from '@/bindings'
import { toast } from 'sonner'
import {
  buildTreeFromEntries,
  type VfsNode,
  type DirectoryEntry,
} from '@/utils/vfs-tree'

interface FileState {
  fileId: string
  metadata: FileMetadata | null
  editors: Editor[]
  activeEditorId: string | null
  blocks: Block[]
  selectedBlockId: string | null
  events: Event[]
  grants: Grant[]
}

interface AppStore {
  // State
  files: Map<string, FileState>
  currentFileId: string | null

  // File operations
  openFile: (path: string) => Promise<string>
  setCurrentFile: (fileId: string | null) => void
  getFileMetadata: (fileId: string) => FileMetadata | null
  getFileInfo: (fileId: string) => Promise<FileMetadata>
  listOpenFiles: () => Promise<string[]>
  saveFile: (fileId: string) => Promise<void>
  createFile: (path: string) => Promise<string>
  renameFile: (fileId: string, newName: string) => Promise<void>
  duplicateFile: (fileId: string) => Promise<string>
  closeFile: (fileId: string) => Promise<void>

  // Block operations
  loadBlocks: (fileId: string) => Promise<void>
  fetchBlock: (fileId: string, blockId: string) => Promise<Block | null>
  getBlocks: (fileId: string) => Block[]
  getBlock: (fileId: string, blockId: string) => Block | undefined
  selectBlock: (blockId: string) => void
  updateBlock: (
    fileId: string,
    blockId: string,
    content: string,
    blockType?: string
  ) => Promise<void>
  createBlock: (
    fileId: string,
    name: string,
    blockType: string,
    source?: string
  ) => Promise<void>
  deleteBlock: (fileId: string, blockId: string) => Promise<void>
  renameBlock: (
    fileId: string,
    blockId: string,
    newName: string
  ) => Promise<void>
  changeBlockType: (
    fileId: string,
    blockId: string,
    blockType?: string,
    fileExtension?: string
  ) => Promise<void>
  updateBlockMetadata: (
    fileId: string,
    blockId: string,
    metadata: Record<string, unknown>
  ) => Promise<void>
  checkPermission: (
    fileId: string,
    blockId: string,
    capability: string
  ) => Promise<boolean>

  // Directory/VFS operations
  getOutlineTree: (fileId: string) => VfsNode[]
  getOutlineRepos: (
    fileId: string
  ) => { blockId: string; name: string; tree: VfsNode[] }[]
  getLinkedRepos: (
    fileId: string
  ) => { blockId: string; name: string; tree: VfsNode[] }[]
  createEntry: (
    fileId: string,
    blockId: string,
    path: string,
    type: 'file' | 'directory',
    options?: { content?: string; block_type?: string; source?: string }
  ) => Promise<void>
  renameEntry: (
    fileId: string,
    blockId: string,
    oldPath: string,
    newPath: string
  ) => Promise<void>
  renameEntryWithTypeChange: (
    fileId: string,
    blockId: string,
    oldPath: string,
    newPath: string,
    blockType?: string,
    fileExtension?: string
  ) => Promise<void>
  deleteEntry: (fileId: string, blockId: string, path: string) => Promise<void>
  importDirectory: (
    fileId: string,
    blockId: string,
    sourcePath: string,
    targetPath?: string
  ) => Promise<void>
  checkoutWorkspace: (
    fileId: string,
    blockId: string,
    targetPath: string,
    sourcePath?: string
  ) => Promise<void>

  // Editor operations
  loadEditors: (fileId: string) => Promise<void>
  createEditor: (
    fileId: string,
    name: string,
    editorType?: string,
    blockId?: string
  ) => Promise<Editor>
  deleteEditor: (
    fileId: string,
    editorId: string,
    blockId?: string
  ) => Promise<void>
  setActiveEditor: (fileId: string, editorId: string) => Promise<void>
  getActiveEditor: (fileId: string) => Editor | undefined
  getEditors: (fileId: string) => Editor[]
  getSystemEditorId: () => Promise<string>

  // Event and Grant operations
  getEvents: (fileId: string) => Event[]
  loadEvents: (fileId: string) => Promise<void>
  restoreToEvent: (
    fileId: string,
    blockId: string,
    eventId: string
  ) => Promise<void>
  getGrants: (fileId: string) => Grant[]
  getBlockGrants: (fileId: string, blockId: string) => Grant[]
  loadGrants: (fileId: string) => Promise<void>
  grantCapability: (
    fileId: string,
    targetEditor: string,
    capability: string,
    targetBlock?: string,
    granterEditorId?: string
  ) => Promise<void>
  revokeCapability: (
    fileId: string,
    targetEditor: string,
    capability: string,
    targetBlock?: string
  ) => Promise<void>

  // Link operations
  linkBlock: (
    fileId: string,
    sourceBlockId: string,
    targetBlockId: string,
    relation?: string
  ) => Promise<void>
  unlinkBlock: (
    fileId: string,
    sourceBlockId: string,
    targetBlockId: string,
    relation?: string
  ) => Promise<void>

  // Task operations
  getTaskBlocks: (fileId: string) => Block[]
  createTaskBlock: (fileId: string, name: string) => Promise<void>
  commitTask: (
    fileId: string,
    taskBlockId: string
  ) => Promise<{
    commit_hash: string
    branch_name: string
    exported_files: string[]
  }>

  // Terminal operations
  createTerminalBlock: (
    fileId: string,
    name: string,
    editorId: string
  ) => Promise<string>
  initTerminal: (
    fileId: string,
    blockId: string,
    cols: number,
    rows: number,
    cwd?: string,
    editorId?: string
  ) => Promise<void>
  writeToPty: (
    fileId: string,
    blockId: string,
    data: string,
    editorId?: string
  ) => Promise<void>
  resizePty: (
    fileId: string,
    blockId: string,
    cols: number,
    rows: number,
    editorId?: string
  ) => Promise<void>
  saveTerminal: (
    fileId: string,
    blockId: string,
    content: string,
    editorId?: string
  ) => Promise<void>
  closeTerminalSession: (
    fileId: string,
    blockId: string,
    editorId?: string
  ) => Promise<void>

  // Agent operations
  createAgent: (
    fileId: string,
    targetProjectId: string,
    name?: string
  ) => Promise<AgentCreateResult>
  enableAgent: (
    fileId: string,
    agentBlockId: string
  ) => Promise<AgentEnableResult>
  disableAgent: (
    fileId: string,
    agentBlockId: string
  ) => Promise<AgentDisableResult>
  getAgentBlocks: (fileId: string) => Block[]

  // Computed state
  selectedBlockId: string | null
}

export const useAppStore = create<AppStore>((set, get) => ({
  // Initial state
  files: new Map(),
  currentFileId: null,
  selectedBlockId: null,

  // File operations
  openFile: async (path: string) => {
    try {
      const fileId = await TauriClient.file.openFile(path)
      const metadata = await TauriClient.file.getFileInfo(fileId)

      const files = new Map(get().files)
      files.set(fileId, {
        fileId,
        metadata,
        editors: [],
        activeEditorId: null,
        blocks: [],
        selectedBlockId: null,
        events: [],
        grants: [],
      })

      set({ files, currentFileId: fileId })

      // Load blocks, editors, and grants
      await get().loadBlocks(fileId)
      await get().loadEditors(fileId)
      await get().loadGrants(fileId)

      return fileId
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to open file: ${errorMessage}`)
      throw error
    }
  },

  setCurrentFile: (fileId: string | null) => {
    set({ currentFileId: fileId })
  },

  getFileMetadata: (fileId: string) => {
    const fileState = get().files.get(fileId)
    return fileState?.metadata || null
  },

  getFileInfo: async (fileId: string) => {
    try {
      return await TauriClient.file.getFileInfo(fileId)
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to get file info: ${errorMessage}`)
      throw error
    }
  },

  saveFile: async (fileId: string) => {
    try {
      await TauriClient.file.saveFile(fileId)
      // Note: Success toast is handled by the caller (EditorCanvas) to avoid duplicate messages
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to save file: ${errorMessage}`)
      throw error
    }
  },

  listOpenFiles: async () => {
    try {
      return await TauriClient.file.listOpenFiles()
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to list open files: ${errorMessage}`)
      throw error
    }
  },

  createFile: async (path: string) => {
    try {
      const fileId = await TauriClient.file.createFile(path)
      // Note: Success toast is handled by the caller
      return fileId
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to create file: ${errorMessage}`)
      throw error
    }
  },

  renameFile: async (fileId: string, newName: string) => {
    try {
      await TauriClient.file.renameFile(fileId, newName)
      // Update metadata in store if file is loaded
      const files = new Map(get().files)
      const fileState = files.get(fileId)
      if (fileState && fileState.metadata) {
        const updatedMetadata = { ...fileState.metadata, name: newName }
        files.set(fileId, { ...fileState, metadata: updatedMetadata })
        set({ files })
      }
      // Note: Success toast is handled by the caller
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to rename file: ${errorMessage}`)
      throw error
    }
  },

  duplicateFile: async (fileId: string) => {
    try {
      const newFileId = await TauriClient.file.duplicateFile(fileId)
      // Note: Success toast is handled by the caller
      return newFileId
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to duplicate file: ${errorMessage}`)
      throw error
    }
  },

  closeFile: async (fileId: string) => {
    try {
      await TauriClient.file.closeFile(fileId)
      // Remove file from store
      const files = new Map(get().files)
      files.delete(fileId)
      // If this was the current file, clear current file
      const currentFileId =
        get().currentFileId === fileId ? null : get().currentFileId
      set({ files, currentFileId })
      // Note: Success toast is handled by the caller
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to close file: ${errorMessage}`)
      throw error
    }
  },

  // Block operations
  loadBlocks: async (fileId: string) => {
    try {
      const blocks = await TauriClient.block.getAllBlocks(fileId)
      const files = new Map(get().files)
      const fileState = files.get(fileId)
      if (fileState) {
        files.set(fileId, { ...fileState, blocks })
        set({ files })
      }
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to load blocks: ${errorMessage}`)
    }
  },

  fetchBlock: async (fileId: string, blockId: string) => {
    try {
      const block = await TauriClient.block.getBlock(fileId, blockId)
      const files = new Map(get().files)
      const fileState = files.get(fileId)
      if (fileState) {
        // Update or add block to the list
        const updatedBlocks = [
          ...fileState.blocks.filter((b) => b.block_id !== blockId),
          block,
        ]
        files.set(fileId, { ...fileState, blocks: updatedBlocks })
        set({ files })
      }
      return block
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      // Only show toast if it's not a "not found" error which might be expected in some flows
      if (!errorMessage.includes('not found')) {
        toast.error(`Failed to fetch block: ${errorMessage}`)
      }
      return null
    }
  },

  getBlocks: (fileId: string) => {
    const fileState = get().files.get(fileId)
    return fileState?.blocks || []
  },

  getBlock: (fileId: string, blockId: string) => {
    const fileState = get().files.get(fileId)
    return fileState?.blocks.find((b) => b.block_id === blockId)
  },

  selectBlock: (blockId: string) => {
    const currentFileId = get().currentFileId
    if (!currentFileId) return

    const files = new Map(get().files)
    const fileState = files.get(currentFileId)
    if (fileState) {
      files.set(currentFileId, { ...fileState, selectedBlockId: blockId })
      set({ files, selectedBlockId: blockId })
    }
  },

  updateBlock: async (
    fileId: string,
    blockId: string,
    content: string,
    blockType: string = 'markdown'
  ) => {
    try {
      await TauriClient.block.writeBlock(fileId, blockId, content, blockType)
      // Reload blocks to get the updated content
      await get().loadBlocks(fileId)
      // Note: Success toast is handled by the caller (EditorCanvas) to avoid duplicate messages
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to update block: ${errorMessage}`)
      throw error
    }
  },

  createBlock: async (
    fileId: string,
    name: string,
    blockType: string,
    source?: string
  ) => {
    try {
      await TauriClient.block.createBlock(
        fileId,
        name,
        blockType,
        undefined,
        source
      )
      // Reload blocks to get the new block
      await get().loadBlocks(fileId)
      toast.success('Block created successfully')
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to create block: ${errorMessage}`)
      throw error
    }
  },

  deleteBlock: async (fileId: string, blockId: string) => {
    try {
      await TauriClient.block.deleteBlock(fileId, blockId)
      // Reload blocks to reflect the deletion
      await get().loadBlocks(fileId)

      // If the deleted block was selected, clear selection
      if (get().selectedBlockId === blockId) {
        get().selectBlock('')
      }

      toast.success('Block deleted successfully')
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to delete block: ${errorMessage}`)
      throw error
    }
  },

  renameBlock: async (fileId: string, blockId: string, newName: string) => {
    try {
      await TauriClient.block.renameBlock(fileId, blockId, newName)
      await get().loadBlocks(fileId)
      toast.success('Block renamed successfully')
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to rename block: ${errorMessage}`)
      throw error
    }
  },

  changeBlockType: async (
    fileId: string,
    blockId: string,
    blockType?: string,
    fileExtension?: string
  ) => {
    try {
      await TauriClient.block.changeBlockType(
        fileId,
        blockId,
        blockType ?? null,
        fileExtension ?? null
      )
      await get().loadBlocks(fileId)
      toast.success('Block type changed successfully')
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to change block type: ${errorMessage}`)
      throw error
    }
  },

  updateBlockMetadata: async (
    fileId: string,
    blockId: string,
    metadata: Record<string, unknown>
  ) => {
    try {
      // Use the new update_block_metadata Tauri command
      await TauriClient.block.updateBlockMetadata(fileId, blockId, metadata)

      // TODO: Performance optimization - Currently reloading ALL blocks just to update one block's metadata.
      // Consider: 1) Backend returning the updated block, or 2) Optimistic update pattern
      // This causes unnecessary overhead when file has many blocks.
      await get().loadBlocks(fileId)

      toast.success('Metadata updated successfully')
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to update metadata: ${errorMessage}`)
      throw error
    }
  },

  checkPermission: async (
    fileId: string,
    blockId: string,
    capability: string
  ) => {
    try {
      const fileState = get().files.get(fileId)
      const activeEditorId = fileState?.activeEditorId || undefined
      return await TauriClient.block.checkPermission(
        fileId,
        blockId,
        capability,
        activeEditorId
      )
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      console.error(`Failed to check permission: ${errorMessage}`)
      return false
    }
  },

  // Directory/VFS operations
  getOutlineTree: (fileId: string) => {
    const blocks = get().getBlocks(fileId)
    // 1. Find the system outline by source='outline'
    // We take the first directory block marked as outline source
    const outlineBlock = blocks.find(
      (b) =>
        b.block_type === 'directory' &&
        (b.contents as any)?.source === 'outline'
    )

    if (!outlineBlock) return []

    const contents = outlineBlock.contents as {
      entries?: Record<string, DirectoryEntry>
    }

    return buildTreeFromEntries(contents.entries || {}, blocks)
  },

  getOutlineRepos: (fileId: string) => {
    const blocks = get().getBlocks(fileId)
    const fileState = get().files.get(fileId)
    const activeEditorId = fileState?.activeEditorId
    const grants = fileState?.grants || []

    // Filter: directory blocks with source='outline'
    const directoryBlocks = blocks.filter(
      (b) =>
        b.block_type === 'directory' &&
        (b.contents as any)?.source === 'outline'
    )

    // Permission filter: only show blocks where active editor has directory.read permission
    const visibleBlocks = directoryBlocks.filter((block) => {
      // If no active editor, don't show any blocks
      if (!activeEditorId) return false

      // Owner always has access
      if (block.owner === activeEditorId) return true

      // Check if active editor has directory.read grant for this block or wildcard
      return grants.some(
        (g) =>
          g.editor_id === activeEditorId &&
          g.cap_id === 'directory.read' &&
          (g.block_id === block.block_id || g.block_id === '*')
      )
    })

    return visibleBlocks.map((block) => {
      const contents = block.contents as {
        entries?: Record<string, DirectoryEntry>
      }
      return {
        blockId: block.block_id,
        name: block.name,
        tree: buildTreeFromEntries(contents.entries || {}, blocks),
      }
    })
  },

  getLinkedRepos: (fileId: string) => {
    const blocks = get().getBlocks(fileId)
    const fileState = get().files.get(fileId)
    const activeEditorId = fileState?.activeEditorId
    const grants = fileState?.grants || []

    // Filter: directory blocks with source='linked'
    const directoryBlocks = blocks.filter(
      (b) =>
        b.block_type === 'directory' && (b.contents as any)?.source === 'linked'
    )

    // Permission filter: only show blocks where active editor has directory.read permission
    const visibleBlocks = directoryBlocks.filter((block) => {
      // If no active editor, don't show any blocks
      if (!activeEditorId) return false

      // Owner always has access
      if (block.owner === activeEditorId) return true

      // Check if active editor has directory.read grant for this block or wildcard
      return grants.some(
        (g) =>
          g.editor_id === activeEditorId &&
          g.cap_id === 'directory.read' &&
          (g.block_id === block.block_id || g.block_id === '*')
      )
    })

    return visibleBlocks.map((block) => {
      const contents = block.contents as {
        entries?: Record<string, DirectoryEntry>
      }
      return {
        blockId: block.block_id,
        name: block.name,
        tree: buildTreeFromEntries(contents.entries || {}, blocks),
      }
    })
  },

  createEntry: async (
    fileId: string,
    blockId: string,
    path: string,
    type: 'file' | 'directory',
    options?: { content?: string; block_type?: string; source?: string }
  ) => {
    try {
      await TauriClient.directory.createEntry(
        fileId,
        blockId,
        path,
        type,
        options
      )
      await get().loadBlocks(fileId)
      toast.success('Entry created successfully')
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to create entry: ${errorMessage}`)
      throw error
    }
  },

  renameEntry: async (
    fileId: string,
    blockId: string,
    oldPath: string,
    newPath: string
  ) => {
    try {
      await TauriClient.directory.renameEntry(fileId, blockId, oldPath, newPath)
      await get().loadBlocks(fileId)
      toast.success('Entry renamed successfully')
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to rename entry: ${errorMessage}`)
      throw error
    }
  },

  renameEntryWithTypeChange: async (
    fileId: string,
    blockId: string,
    oldPath: string,
    newPath: string,
    blockType?: string,
    fileExtension?: string
  ) => {
    try {
      await TauriClient.directory.renameEntryWithTypeChange(
        fileId,
        blockId,
        oldPath,
        newPath,
        blockType,
        fileExtension
      )
      await get().loadBlocks(fileId)
      toast.success('Entry renamed and type changed successfully')
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to rename entry with type change: ${errorMessage}`)
      throw error
    }
  },

  deleteEntry: async (fileId: string, blockId: string, path: string) => {
    try {
      await TauriClient.directory.deleteEntry(fileId, blockId, path)
      await get().loadBlocks(fileId)
      toast.success('Entry deleted successfully')
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to delete entry: ${errorMessage}`)
      throw error
    }
  },

  importDirectory: async (
    fileId: string,
    blockId: string,
    sourcePath: string,
    targetPath?: string
  ) => {
    try {
      await TauriClient.directory.importDirectory(
        fileId,
        blockId,
        sourcePath,
        targetPath
      )
      await get().loadBlocks(fileId)
      toast.success('Directory imported successfully')

      // Auto-inject git hooks for linked repos with .git (commit protect ON)
      // Hooks are stored in .elf temp dir — disappear on crash/close
      try {
        await TauriClient.task.injectHooksForRepo(fileId, sourcePath)
        toast.info('Commit protect enabled for tracked commits')
      } catch {
        // Not a git repo or injection failed — silently ignore
      }
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to import directory: ${errorMessage}`)
      throw error
    }
  },

  checkoutWorkspace: async (
    fileId: string,
    blockId: string,
    targetPath: string,
    sourcePath?: string
  ) => {
    try {
      await TauriClient.directory.checkoutWorkspace(
        fileId,
        blockId,
        targetPath,
        sourcePath
      )
      toast.success('Workspace checked out successfully')
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to checkout workspace: ${errorMessage}`)
      throw error
    }
  },

  // Editor operations
  loadEditors: async (fileId: string) => {
    try {
      const editors = await TauriClient.editor.listEditors(fileId)
      let activeEditorId = await TauriClient.editor.getActiveEditor(fileId)

      // Validate activeEditorId - if it doesn't exist in editors list, reset to first editor
      if (
        activeEditorId &&
        !editors.some((e) => e.editor_id === activeEditorId)
      ) {
        console.warn(
          `Active editor ${activeEditorId} not found in editors list. Resetting to first editor.`
        )
        if (editors.length > 0) {
          // Prefer System editor, otherwise use first editor
          const systemEditor = editors.find((e) => e.name === 'System')
          activeEditorId = systemEditor
            ? systemEditor.editor_id
            : editors[0].editor_id
          // Update backend active editor
          await TauriClient.editor.setActiveEditor(fileId, activeEditorId)
        } else {
          activeEditorId = null
        }
      }

      const files = new Map(get().files)
      const fileState = files.get(fileId)
      if (fileState) {
        files.set(fileId, { ...fileState, editors, activeEditorId })
        set({ files })
      }
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to load editors: ${errorMessage}`)
    }
  },

  createEditor: async (
    fileId: string,
    name: string,
    editorType?: string,
    blockId?: string
  ) => {
    try {
      const newEditor = await TauriClient.editor.createEditor(
        fileId,
        name,
        editorType,
        blockId
      )
      // Reload editors to get the updated list
      await get().loadEditors(fileId)
      toast.success(`User "${name}" created successfully`)
      return newEditor
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to create user: ${errorMessage}`)
      throw error
    }
  },

  deleteEditor: async (fileId: string, editorId: string, blockId?: string) => {
    try {
      await TauriClient.editor.deleteEditor(fileId, editorId, blockId)
      // Reload editors to get the updated list
      await get().loadEditors(fileId)
      // Also reload grants since deleting an editor removes all their grants
      await get().loadGrants(fileId)
      toast.success('Collaborator removed successfully')
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to remove collaborator: ${errorMessage}`)
      throw error
    }
  },

  setActiveEditor: async (fileId: string, editorId: string) => {
    try {
      await TauriClient.editor.setActiveEditor(fileId, editorId)
      const files = new Map(get().files)
      const fileState = files.get(fileId)
      if (fileState) {
        files.set(fileId, { ...fileState, activeEditorId: editorId })
        set({ files })
      }

      // CRITICAL: Clear selected block when switching users to prevent unauthorized access
      set({ selectedBlockId: null })

      // Reload all data for the new user context
      // The backend will filter these based on the new active editor's permissions
      await Promise.all([
        get().loadBlocks(fileId),
        get().loadGrants(fileId),
        get().loadEvents(fileId),
        get().loadEditors(fileId),
      ])

      toast.success('Switched user - permissions updated')
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to set active editor: ${errorMessage}`)
    }
  },

  getActiveEditor: (fileId: string) => {
    const fileState = get().files.get(fileId)
    if (!fileState?.activeEditorId) return undefined
    return fileState.editors.find(
      (e) => e.editor_id === fileState.activeEditorId
    )
  },

  getEditors: (fileId: string) => {
    const fileState = get().files.get(fileId)
    return fileState?.editors || []
  },

  getSystemEditorId: async () => {
    try {
      return await TauriClient.file.getSystemEditorId()
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to get system editor ID: ${errorMessage}`)
      throw error
    }
  },

  // Event and Grant operations
  getEvents: (fileId: string) => {
    const fileState = get().files.get(fileId)
    return fileState?.events || []
  },

  loadEvents: async (fileId: string) => {
    try {
      const events = await TauriClient.event.getAllEvents(fileId)
      const files = new Map(get().files)
      const fileState = files.get(fileId)
      if (fileState) {
        files.set(fileId, { ...fileState, events })
        set({ files })
      }
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to load events: ${errorMessage}`)
    }
  },

  /**
   * 回溯到指定事件时刻的内容
   * 直接替换编辑器内容，用户可继续编辑和保存
   */
  restoreToEvent: async (fileId: string, blockId: string, eventId: string) => {
    try {
      // 1. 获取历史状态 (包含 name, content, metadata 以及权限 grants 等)
      const { block: historicalBlock, grants: historicalGrants } =
        await TauriClient.event.getStateAtEvent(fileId, blockId, eventId)

      // 2. 更新当前 block 的状态和权限（仅在内存中，不保存到数据库）
      const files = new Map(get().files)
      const fileState = files.get(fileId)
      if (fileState) {
        // 更新 blocks
        const updatedBlocks = fileState.blocks.map((block) => {
          if (block.block_id === blockId) {
            // 完整替换 block 状态，恢复 name, content 和 metadata
            return {
              ...historicalBlock,
            }
          }
          return block
        })

        // 更新 fileState，同时包含新的 grants
        files.set(fileId, {
          ...fileState,
          blocks: updatedBlocks,
          grants: historicalGrants,
        })
        set({ files })
      }

      // 3. 触发通知
      toast.success('已恢复到历史快照，包含描述、标题和权限')
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to restore: ${errorMessage}`)
      throw error
    }
  },

  getGrants: (fileId: string) => {
    const fileState = get().files.get(fileId)
    return fileState?.grants || []
  },

  getBlockGrants: (fileId: string, blockId: string) => {
    const fileState = get().files.get(fileId)
    if (!fileState) return []
    return fileState.grants.filter(
      (g) => g.block_id === blockId || g.block_id === '*'
    )
  },

  loadGrants: async (fileId: string) => {
    try {
      const grants = await TauriClient.editor.listGrants(fileId)
      const files = new Map(get().files)
      const fileState = files.get(fileId)
      if (fileState) {
        files.set(fileId, { ...fileState, grants })
        set({ files })
      }
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to load grants: ${errorMessage}`)
    }
  },

  grantCapability: async (
    fileId: string,
    targetEditor: string,
    capability: string,
    targetBlock: string = '*',
    granterEditorId?: string
  ) => {
    try {
      await TauriClient.editor.grantCapability(
        fileId,
        targetEditor,
        capability,
        targetBlock,
        granterEditorId
      )
      // Reload grants and events to update UI
      await get().loadGrants(fileId)
      await get().loadEvents(fileId)
      toast.success('Permission granted')
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to grant permission: ${errorMessage}`)
      throw error
    }
  },

  revokeCapability: async (
    fileId: string,
    targetEditor: string,
    capability: string,
    targetBlock: string = '*'
  ) => {
    try {
      await TauriClient.editor.revokeCapability(
        fileId,
        targetEditor,
        capability,
        targetBlock
      )
      // Reload grants and events to update UI
      await get().loadGrants(fileId)
      await get().loadEvents(fileId)
      toast.success('Permission revoked')
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to revoke permission: ${errorMessage}`)
      throw error
    }
  },

  // Link operations
  linkBlock: async (
    fileId: string,
    sourceBlockId: string,
    targetBlockId: string,
    relation: string = 'implement'
  ) => {
    try {
      const activeEditorId =
        get().files.get(fileId)?.activeEditorId || undefined
      const editorId =
        activeEditorId || (await TauriClient.file.getSystemEditorId())

      const cmd = {
        cmd_id: crypto.randomUUID(),
        editor_id: editorId,
        cap_id: 'core.link',
        block_id: sourceBlockId,
        payload: {
          relation,
          target_id: targetBlockId,
        } as unknown as import('@/bindings').JsonValue,
        timestamp: new Date().toISOString(),
      }
      await TauriClient.block.executeCommand(fileId, cmd)
      await get().loadBlocks(fileId)
      toast.success('Block linked')
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to link block: ${errorMessage}`)
      throw error
    }
  },

  unlinkBlock: async (
    fileId: string,
    sourceBlockId: string,
    targetBlockId: string,
    relation: string = 'implement'
  ) => {
    try {
      const activeEditorId =
        get().files.get(fileId)?.activeEditorId || undefined
      const editorId =
        activeEditorId || (await TauriClient.file.getSystemEditorId())

      const cmd = {
        cmd_id: crypto.randomUUID(),
        editor_id: editorId,
        cap_id: 'core.unlink',
        block_id: sourceBlockId,
        payload: {
          relation,
          target_id: targetBlockId,
        } as unknown as import('@/bindings').JsonValue,
        timestamp: new Date().toISOString(),
      }
      await TauriClient.block.executeCommand(fileId, cmd)
      await get().loadBlocks(fileId)
      toast.success('Block unlinked')
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to unlink block: ${errorMessage}`)
      throw error
    }
  },

  // Task operations
  getTaskBlocks: (fileId: string) => {
    const fileState = get().files.get(fileId)
    return (fileState?.blocks || []).filter((b) => b.block_type === 'task')
  },

  createTaskBlock: async (fileId: string, name: string) => {
    try {
      await TauriClient.block.createBlock(fileId, name, 'task')
      await get().loadBlocks(fileId)
      toast.success('Task created successfully')
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to create task: ${errorMessage}`)
      throw error
    }
  },

  commitTask: async (fileId: string, taskBlockId: string) => {
    try {
      const result = await TauriClient.task.commitTask(fileId, taskBlockId)
      await get().loadBlocks(fileId)
      await get().loadEvents(fileId)
      toast.success(
        `Task committed to branch ${result.branch_name} (${result.commit_hash.slice(0, 7)})`
      )
      return result
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to commit task: ${errorMessage}`)
      throw error
    }
  },

  // Terminal operations
  createTerminalBlock: async (
    fileId: string,
    name: string,
    editorId: string
  ) => {
    try {
      const events = await TauriClient.block.createBlock(
        fileId,
        name,
        'terminal',
        editorId,
        'terminal'
      )
      const blockId = events[0].entity
      await get().loadBlocks(fileId)
      return blockId
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to create terminal block: ${errorMessage}`)
      throw error
    }
  },

  initTerminal: async (
    fileId: string,
    blockId: string,
    cols: number,
    rows: number,
    cwd?: string,
    editorId?: string
  ) => {
    try {
      await TauriClient.terminal.initTerminal(
        fileId,
        blockId,
        cols,
        rows,
        cwd,
        editorId
      )
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to initialize terminal: ${errorMessage}`)
      throw error
    }
  },

  writeToPty: async (
    fileId: string,
    blockId: string,
    data: string,
    editorId?: string
  ) => {
    try {
      await TauriClient.terminal.writeToPty(fileId, blockId, data, editorId)
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      console.error(`Failed to write to PTY: ${errorMessage}`)
      throw error
    }
  },

  resizePty: async (
    fileId: string,
    blockId: string,
    cols: number,
    rows: number,
    editorId?: string
  ) => {
    try {
      await TauriClient.terminal.resizePty(
        fileId,
        blockId,
        cols,
        rows,
        editorId
      )
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      console.error(`Failed to resize PTY: ${errorMessage}`)
      throw error
    }
  },

  saveTerminal: async (
    fileId: string,
    blockId: string,
    content: string,
    editorId?: string
  ) => {
    try {
      await TauriClient.terminal.saveTerminal(
        fileId,
        blockId,
        content,
        editorId
      )

      // Update local state so that if we switch back, we have the content
      const files = new Map(get().files)
      const fileState = files.get(fileId)
      if (fileState) {
        const blocks = [...fileState.blocks]
        const blockIndex = blocks.findIndex((b) => b.block_id === blockId)
        if (blockIndex !== -1) {
          // Deep clone the block to avoid mutation issues
          const updatedBlock = { ...blocks[blockIndex] }
          // Ensure contents object exists and update saved_content
          updatedBlock.contents = {
            ...(updatedBlock.contents as object),
            saved_content: content,
          }
          blocks[blockIndex] = updatedBlock

          files.set(fileId, { ...fileState, blocks })
          set({ files })
        }
      }
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      console.error(`Failed to save terminal: ${errorMessage}`)
      // Don't throw here to avoid disrupting the UI for background saves
    }
  },

  closeTerminalSession: async (
    fileId: string,
    blockId: string,
    editorId?: string
  ) => {
    try {
      await TauriClient.terminal.closeTerminalSession(fileId, blockId, editorId)
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      console.error(`Failed to close terminal session: ${errorMessage}`)
      throw error
    }
  },

  // Agent operations
  createAgent: async (
    fileId: string,
    targetProjectId: string,
    name?: string
  ) => {
    try {
      const payload: AgentCreateV2Payload = {
        target_project_id: targetProjectId,
        name: name || null,
      }
      const result = await TauriClient.agent.createAgent(fileId, payload)
      // Reload blocks to reflect the new Agent Block
      await get().loadBlocks(fileId)
      toast.success(result.message)
      if (result.needs_restart) {
        toast.info('Please restart Claude Code to apply changes')
      }
      return result
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to create agent: ${errorMessage}`)
      throw error
    }
  },

  enableAgent: async (fileId: string, agentBlockId: string) => {
    try {
      const result = await TauriClient.agent.enableAgent(fileId, agentBlockId)
      // Reload blocks to reflect updated agent status
      await get().loadBlocks(fileId)
      toast.success(result.message)
      if (result.warnings.length > 0) {
        result.warnings.forEach((w) => toast.warning(w))
      }
      if (result.needs_restart) {
        toast.info('Please restart Claude Code to apply changes')
      }
      return result
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to enable agent: ${errorMessage}`)
      throw error
    }
  },

  disableAgent: async (fileId: string, agentBlockId: string) => {
    try {
      const result = await TauriClient.agent.disableAgent(fileId, agentBlockId)
      // Reload blocks to reflect updated agent status
      await get().loadBlocks(fileId)
      toast.success(result.message)
      if (result.warnings.length > 0) {
        result.warnings.forEach((w) => toast.warning(w))
      }
      return result
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)
      toast.error(`Failed to disable agent: ${errorMessage}`)
      throw error
    }
  },

  getAgentBlocks: (fileId: string) => {
    const blocks = get().getBlocks(fileId)
    return blocks.filter((b) => b.block_type === 'agent')
  },
}))
