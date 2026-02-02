import { useState, useMemo } from 'react'
import { Link } from 'react-router-dom'
import {
  Plus,
  MoreHorizontal,
  Download,
  FileUp,
  FolderUp,
  Trash2,
} from 'lucide-react'
import { cn } from '@/lib/utils'
import { useAppStore } from '@/lib/app-store'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Separator } from '@/components/ui/separator'
import { ImportRepositoryModal } from './ImportRepositoryModal'
import { VfsTree } from './VfsTree'
import { CreateEntryDialog } from './CreateEntryDialog'
import { AddWorkdirDialog } from './AddWorkdirDialog'
import type { VfsNode } from '@/utils/vfs-tree'
import { toast } from 'sonner'
import { open } from '@tauri-apps/plugin-dialog'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'

export const FilePanel = () => {
  const {
    currentFileId,
    selectedBlockId,
    selectBlock,
    getOutlineRepos,
    getLinkedRepos,
    getBlocks,
    getTaskBlocks,
    createEntry,
    renameEntry,
    renameEntryWithTypeChange,
    deleteEntry,
    importDirectory,
    checkoutWorkspace,
    createBlock,
    createTaskBlock,
    deleteBlock,
    renameBlock,
    checkoutWorkspace: checkoutWorkspaceAction,
    files,
  } = useAppStore()
  const [isImportModalOpen, setIsImportModalOpen] = useState(false)
  const [showAddWorkdirDialog, setShowAddWorkdirDialog] = useState(false)
  const [createDialog, setCreateDialog] = useState({
    open: false,
    directoryBlockId: '',
    parentPath: '',
    type: 'file' as 'file' | 'directory',
    source: 'outline' as 'outline' | 'linked',
  })

  // No need to initialize system outline - users create their own outline blocks

  // Selectors for trees - Memoized to prevent redundant sorting/building
  const outlineRepos = useMemo(
    () => (currentFileId ? getOutlineRepos(currentFileId) : []),
    [currentFileId, getOutlineRepos, files]
  )

  const linkedRepos = useMemo(
    () => (currentFileId ? getLinkedRepos(currentFileId) : []),
    [currentFileId, getLinkedRepos, files]
  )

  const taskBlocks = useMemo(
    () => (currentFileId ? getTaskBlocks(currentFileId) : []),
    [currentFileId, getTaskBlocks, files]
  )

  // --- Handlers ---

  const handleSelect = (node: VfsNode) => {
    if (node.blockId) {
      selectBlock(node.blockId)
    }
  }

  const openCreateDialog = (
    directoryBlockId: string,
    parentPath: string,
    type: 'file' | 'directory',
    source: 'outline' | 'linked'
  ) => {
    setCreateDialog({
      open: true,
      directoryBlockId,
      parentPath,
      type,
      source,
    })
  }

  const handleCreateConfirm = async (name: string) => {
    if (!currentFileId) return

    const { directoryBlockId, parentPath, type, source } = createDialog
    const path = parentPath ? `${parentPath}/${name}` : name

    try {
      await createEntry(currentFileId, directoryBlockId, path, type, {
        source,
        // Note: block_type will be inferred by backend based on file extension
      })
      // Note: toast is already shown by the createEntry Action
    } catch (error) {
      // Error toast is already shown by the Action
    }
  }

  const handleRename = async (
    directoryBlockId: string,
    node: VfsNode,
    newName: string
  ) => {
    if (!currentFileId) return

    const newPath = node.path.includes('/')
      ? `${node.path.substring(0, node.path.lastIndexOf('/'))}/${newName}`
      : newName

    try {
      // Check if extension changed for files
      if (node.type === 'file' && node.blockId) {
        const oldExt = node.name.includes('.') ? node.name.split('.').pop() : ''
        const newExt = newName.includes('.') ? newName.split('.').pop() : ''

        if (oldExt !== newExt) {
          // Extension changed - prompt user to confirm atomic rename+type change
          // User confirmed - use atomic operation: rename + type change
          await renameEntryWithTypeChange(
            currentFileId,
            directoryBlockId,
            node.path,
            newPath,
            undefined,
            newExt
          )
          return
        }
      }

      // No extension change - simple rename
      await renameEntry(currentFileId, directoryBlockId, node.path, newPath)
    } catch (error) {
      // Error toast is already shown by the Action
    }
  }

  const handleDelete = async (directoryBlockId: string, node: VfsNode) => {
    if (!currentFileId) return

    const confirmed = confirm(
      `Are you sure you want to delete "${node.name}"? This will remove the reference from this directory.`
    )
    if (!confirmed) return

    try {
      await deleteEntry(currentFileId, directoryBlockId, node.path)
      // Note: toast is already shown by the deleteEntry Action
    } catch (error) {
      // Error toast is already shown by the Action
    }
  }

  // Handle deleting an entire directory block (e.g., whole repo)
  const handleDeleteRepo = async (blockId: string, name: string) => {
    if (!currentFileId) return

    const confirmed = confirm(
      `Are you sure you want to delete "${name}"? This will remove the entire repository and all its contents.`
    )
    if (!confirmed) return

    try {
      await deleteBlock(currentFileId, blockId)
      // Note: toast is already shown by the deleteBlock Action
    } catch (error) {
      // Error toast is already shown by the Action
    }
  }

  const handleExport = async (directoryBlockId: string, node: VfsNode) => {
    if (!currentFileId) return

    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: `Select Export Destination for ${node.name}`,
      })

      if (selected && typeof selected === 'string') {
        await checkoutWorkspace(
          currentFileId,
          directoryBlockId,
          selected,
          node.path !== '' ? node.path : undefined
        )
        // Note: toast is already shown by the checkoutWorkspace Action
      }
    } catch (error) {
      // Error toast is already shown by the Action
    }
  }

  const handleImport = async (isFolder: boolean) => {
    if (!currentFileId) return

    try {
      const sourcePath = await open({
        directory: isFolder,
        multiple: false,
        title: isFolder ? 'Select Folder to Import' : 'Select File to Import',
      })

      if (!sourcePath || typeof sourcePath !== 'string') return

      const itemName = sourcePath.split(/[\\/]/).pop() || 'Imported'

      // Check for name collision in top-level directory blocks
      const existingNames = linkedRepos.map((r) => r.name)
      let uniqueName = itemName
      let counter = 1
      while (existingNames.includes(uniqueName)) {
        uniqueName = `${itemName} (${counter++})`
      }

      // 1. Create the Directory Block
      await createBlock(currentFileId, uniqueName, 'directory')

      // 2. Get the newly created block ID
      const blocks = getBlocks(currentFileId)
      const newBlock = blocks.find(
        (b) => b.name === uniqueName && b.block_type === 'directory'
      )
      if (!newBlock) {
        throw new Error('Failed to create directory block')
      }
      const newBlockId = newBlock.block_id

      // 3. Import the directory contents
      await importDirectory(currentFileId, newBlockId, sourcePath, undefined)

      // Note: toast is already shown by the Actions
    } catch (error) {
      toast.error(`Import failed: ${error}`)
    }
  }

  const handleCreateTask = async () => {
    if (!currentFileId) return
    try {
      await createTaskBlock(currentFileId, 'New Task')
    } catch {
      // Error toast already shown
    }
  }

  const handleRenameTask = async (blockId: string, newName: string) => {
    if (!currentFileId) return
    try {
      await renameBlock(currentFileId, blockId, newName)
    } catch {
      // Error toast already shown
    }
  }

  const handleExportTask = async (blockId: string, name: string) => {
    if (!currentFileId) return
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: `Select Export Destination for ${name}`,
      })
      if (selected && typeof selected === 'string') {
        await checkoutWorkspaceAction(currentFileId, blockId, selected)
      }
    } catch {
      // Error toast already shown
    }
  }

  const handleAddWorkdir = async (name: string) => {
    if (!currentFileId) return

    try {
      // Create a new outline directory block
      await createBlock(currentFileId, name, 'directory', 'outline')
      // Note: toast is already shown by the createBlock Action
    } catch (error) {
      // Error toast is already shown by the Action
      throw error // Re-throw to let dialog handle loading state
    }
  }

  return (
    <aside className="flex h-full min-h-0 w-full flex-col overflow-hidden bg-secondary/30">
      {/* Breadcrumbs */}
      <div className="shrink-0 border-b border-border px-4 py-3">
        <nav className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <Link
            to="/"
            className="transition-colors hover:text-foreground hover:underline"
          >
            Projects
          </Link>
          <span>/</span>
          <span className="truncate font-medium text-foreground">
            {currentFileId ? 'Document Editor' : 'No Project Open'}
          </span>
        </nav>
      </div>

      <ScrollArea className="min-h-0 flex-1">
        <div className="py-2">
          {/* OUTLINE Section */}
          <div className="mb-3 px-2">
            <div className="mb-2 flex items-center justify-between px-1">
              <span className="whitespace-nowrap text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                Outline
              </span>
            </div>
            <div className="space-y-4">
              {outlineRepos.map((repo) => (
                <div key={repo.blockId} className="space-y-1">
                  <div className="group flex items-center justify-between px-1">
                    <span
                      className={cn(
                        'cursor-pointer truncate rounded-sm px-1 text-xs font-medium transition-colors',
                        selectedBlockId === repo.blockId
                          ? 'bg-accent/10 text-accent'
                          : 'text-muted-foreground/80 hover:bg-muted hover:text-foreground'
                      )}
                      onClick={() => selectBlock(repo.blockId)}
                    >
                      {repo.name}
                    </span>
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <button className="rounded p-0.5 text-muted-foreground opacity-0 hover:bg-muted group-hover:opacity-100">
                          <MoreHorizontal className="h-3 w-3" />
                        </button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuItem
                          onClick={() =>
                            openCreateDialog(
                              repo.blockId,
                              '',
                              'file',
                              'outline'
                            )
                          }
                        >
                          <FileUp className="mr-2 h-3.5 w-3.5" />
                          Add Document
                        </DropdownMenuItem>
                        <DropdownMenuItem
                          onClick={() =>
                            openCreateDialog(
                              repo.blockId,
                              '',
                              'directory',
                              'outline'
                            )
                          }
                        >
                          <FolderUp className="mr-2 h-3.5 w-3.5" />
                          Add Folder
                        </DropdownMenuItem>
                        <DropdownMenuItem
                          onClick={() =>
                            handleExport(repo.blockId, {
                              path: '',
                              name: repo.name,
                              type: 'directory',
                              blockId: null,
                              source: 'outline',
                              children: [],
                            })
                          }
                        >
                          <Download className="mr-2 h-3.5 w-3.5" />
                          Export
                        </DropdownMenuItem>
                        <DropdownMenuItem
                          onClick={() =>
                            handleDeleteRepo(repo.blockId, repo.name)
                          }
                          className="text-destructive focus:text-destructive"
                        >
                          <Trash2 className="mr-2 h-3.5 w-3.5" />
                          Delete
                        </DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </div>
                  <VfsTree
                    nodes={repo.tree}
                    activeBlockId={selectedBlockId}
                    onSelect={handleSelect}
                    onAddChild={(node, type) =>
                      openCreateDialog(repo.blockId, node.path, type, 'outline')
                    }
                    onRename={(node, newName) =>
                      handleRename(repo.blockId, node, newName)
                    }
                    onDelete={(node) => handleDelete(repo.blockId, node)}
                    onExport={(node) => handleExport(repo.blockId, node)}
                  />
                </div>
              ))}
            </div>
          </div>

          <Separator className="my-2" />

          {/* TASKS Section */}
          <div className="mb-3 px-2">
            <div className="mb-2 flex items-center justify-between px-1">
              <span className="whitespace-nowrap text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                Tasks
              </span>
              <Button
                size="sm"
                variant="ghost"
                className="h-6 w-6 p-0 text-muted-foreground hover:text-foreground"
                onClick={handleCreateTask}
              >
                <Plus className="h-3.5 w-3.5" />
              </Button>
            </div>
            <div className="space-y-0.5">
              {taskBlocks.map((block) => {
                const isSelected = selectedBlockId === block.block_id

                return (
                  <div
                    key={block.block_id}
                    className={cn(
                      'group flex cursor-pointer items-center gap-2 rounded-md px-2 py-1.5 transition-colors',
                      isSelected
                        ? 'bg-accent/10 text-accent'
                        : 'text-foreground hover:bg-muted/50'
                    )}
                    onClick={() => selectBlock(block.block_id)}
                  >
                    {/* Title */}
                    <span className="min-w-0 flex-1 truncate text-sm">
                      {block.name || 'Untitled'}
                    </span>

                    {/* Actions */}
                    <div className="flex flex-shrink-0 items-center gap-0.5 opacity-0 group-hover:opacity-100">
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <button
                            className="flex h-5 w-5 items-center justify-center rounded text-muted-foreground hover:bg-muted hover:text-foreground"
                            onClick={(e) => e.stopPropagation()}
                          >
                            <MoreHorizontal className="h-3 w-3" />
                          </button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end" className="w-40">
                          <DropdownMenuItem
                            onClick={(e) => {
                              e.stopPropagation()
                              const newName = prompt('Rename task:', block.name)
                              if (newName && newName !== block.name) {
                                handleRenameTask(block.block_id, newName)
                              }
                            }}
                          >
                            Rename
                          </DropdownMenuItem>
                          <DropdownMenuItem
                            onClick={(e) => {
                              e.stopPropagation()
                              handleExportTask(block.block_id, block.name)
                            }}
                          >
                            <Download className="mr-2 h-3.5 w-3.5" />
                            Export
                          </DropdownMenuItem>
                          <DropdownMenuItem
                            onClick={(e) => {
                              e.stopPropagation()
                              handleDeleteRepo(block.block_id, block.name)
                            }}
                            className="text-destructive focus:text-destructive"
                          >
                            <Trash2 className="mr-2 h-3.5 w-3.5" />
                            Delete
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </div>
                  </div>
                )
              })}
              {taskBlocks.length === 0 && (
                <p className="px-1 py-2 text-xs text-muted-foreground/60">
                  No tasks yet
                </p>
              )}
            </div>
          </div>

          <Separator className="my-2" />

          {/* LINKED REPOSITORIES Section */}
          <div className="px-2">
            <div className="mb-2 flex items-center justify-between px-1">
              <span className="whitespace-nowrap text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                Linked Repos
              </span>
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button
                    size="sm"
                    variant="ghost"
                    className="h-6 w-6 flex-shrink-0 p-0 text-muted-foreground hover:text-foreground"
                  >
                    <Plus className="h-3.5 w-3.5" />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end">
                  <DropdownMenuItem onClick={() => handleImport(false)}>
                    <FileUp className="mr-2 h-3.5 w-3.5" />
                    Import File
                  </DropdownMenuItem>
                  <DropdownMenuItem onClick={() => handleImport(true)}>
                    <FolderUp className="mr-2 h-3.5 w-3.5" />
                    Import Folder
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </div>
            <div className="space-y-4">
              {linkedRepos.map((repo) => (
                <div key={repo.blockId} className="space-y-1">
                  <div className="group flex items-center justify-between px-1">
                    <span
                      className={cn(
                        'cursor-pointer truncate rounded-sm px-1 text-xs font-medium transition-colors',
                        selectedBlockId === repo.blockId
                          ? 'bg-accent/10 text-accent'
                          : 'text-muted-foreground/80 hover:bg-muted hover:text-foreground'
                      )}
                      onClick={() => selectBlock(repo.blockId)}
                    >
                      {repo.name}
                    </span>
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <button className="rounded p-0.5 text-muted-foreground opacity-0 hover:bg-muted group-hover:opacity-100">
                          <MoreHorizontal className="h-3 w-3" />
                        </button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuItem
                          onClick={() =>
                            openCreateDialog(repo.blockId, '', 'file', 'linked')
                          }
                        >
                          <FileUp className="mr-2 h-3.5 w-3.5" />
                          Add File
                        </DropdownMenuItem>
                        <DropdownMenuItem
                          onClick={() =>
                            openCreateDialog(
                              repo.blockId,
                              '',
                              'directory',
                              'linked'
                            )
                          }
                        >
                          <FolderUp className="mr-2 h-3.5 w-3.5" />
                          Add Folder
                        </DropdownMenuItem>
                        <DropdownMenuItem
                          onClick={() =>
                            handleExport(repo.blockId, {
                              path: '',
                              name: repo.name,
                              type: 'directory',
                              blockId: null,
                              source: 'linked',
                              children: [],
                            })
                          }
                        >
                          <Download className="mr-2 h-3.5 w-3.5" />
                          Export
                        </DropdownMenuItem>
                        <DropdownMenuItem
                          onClick={() =>
                            handleDeleteRepo(repo.blockId, repo.name)
                          }
                          className="text-destructive focus:text-destructive"
                        >
                          <Trash2 className="mr-2 h-3.5 w-3.5" />
                          Delete
                        </DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </div>
                  <VfsTree
                    nodes={repo.tree}
                    activeBlockId={selectedBlockId}
                    onSelect={handleSelect}
                    onAddChild={(node, type) =>
                      openCreateDialog(repo.blockId, node.path, type, 'linked')
                    }
                    onRename={(node, newName) =>
                      handleRename(repo.blockId, node, newName)
                    }
                    onDelete={(node) => handleDelete(repo.blockId, node)}
                    onExport={(node) => handleExport(repo.blockId, node)}
                  />
                </div>
              ))}
            </div>
          </div>
        </div>
      </ScrollArea>

      <CreateEntryDialog
        open={createDialog.open}
        onOpenChange={(open) => setCreateDialog((prev) => ({ ...prev, open }))}
        onConfirm={handleCreateConfirm}
        type={createDialog.type}
        parentPath={createDialog.parentPath}
        source={createDialog.source}
      />

      <ImportRepositoryModal
        open={isImportModalOpen}
        onOpenChange={setIsImportModalOpen}
        onImport={(_name, source) => {
          if (source === 'Local File') handleImport(false)
          else handleImport(true)
        }}
      />

      <AddWorkdirDialog
        open={showAddWorkdirDialog}
        onOpenChange={setShowAddWorkdirDialog}
        onConfirm={handleAddWorkdir}
      />
    </aside>
  )
}
