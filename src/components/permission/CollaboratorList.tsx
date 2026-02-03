import { useState, useMemo, useEffect, useCallback } from 'react'
import { FolderOpen, UserPlus, Users } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { open } from '@tauri-apps/plugin-dialog'
import { useAppStore } from '@/lib/app-store'
import { CollaboratorItem } from './CollaboratorItem'
import { AddCollaboratorDialog } from './AddCollaboratorDialog'
import { toast } from 'sonner'
import type { AgentContents, Block, Editor } from '@/bindings'

interface CollaboratorListProps {
  fileId: string
  blockId: string
  block: Block
}

export const CollaboratorList = ({
  fileId,
  blockId,
  block,
}: CollaboratorListProps) => {
  const [showAddDialog, setShowAddDialog] = useState(false)
  const [agentConfigDialog, setAgentConfigDialog] = useState<{
    open: boolean
    editorId: string
    editorName: string
    configDir: string
  }>({ open: false, editorId: '', editorName: '', configDir: '' })

  // Fetch system editor ID (file owner from ~/.elf/config.json)
  const [systemEditorId, setSystemEditorId] = useState<string | null>(null)
  const getSystemEditorId = useAppStore((state) => state.getSystemEditorId)

  useEffect(() => {
    getSystemEditorId()
      .then(setSystemEditorId)
      .catch(() => {})
  }, [getSystemEditorId])

  // Subscribe to store state changes
  const editors = useAppStore((state) => {
    const fileState = state.files.get(fileId)
    return fileState?.editors || []
  })
  const grants = useAppStore((state) => {
    const fileState = state.files.get(fileId)
    return fileState?.grants || []
  })
  const activeEditor = useAppStore((state) => {
    const fileState = state.files.get(fileId)
    if (!fileState?.activeEditorId) return undefined
    return fileState.editors.find(
      (e) => e.editor_id === fileState.activeEditorId
    )
  })
  const blocks = useAppStore((state) => {
    const fileState = state.files.get(fileId)
    return fileState?.blocks || []
  })
  const grantCapability = useAppStore((state) => state.grantCapability)
  const revokeCapability = useAppStore((state) => state.revokeCapability)
  const checkPermission = useAppStore((state) => state.checkPermission)
  const createAgent = useAppStore((state) => state.createAgent)
  const enableAgent = useAppStore((state) => state.enableAgent)
  const disableAgent = useAppStore((state) => state.disableAgent)
  const isGlobalCollaborator = useAppStore(
    (state) => state.isGlobalCollaborator
  )

  // Filter agent blocks for matching bot editors to their agent blocks
  const agentBlocks = useMemo(
    () => blocks.filter((b) => b.block_type === 'agent'),
    [blocks]
  )

  // Find the agent block associated with a bot editor (matched by editor_id)
  const findAgentBlockForEditor = useCallback(
    (editor: Editor): Block | undefined => {
      if (editor.editor_type !== 'Bot') return undefined
      return agentBlocks.find((block) => {
        const contents = block.contents as AgentContents | undefined
        return contents?.editor_id === editor.editor_id
      })
    },
    [agentBlocks]
  )

  // Handler for creating an agent for a bot editor (when no agent block exists yet)
  // Opens a dialog to collect config_dir before calling createAgent
  const handleCreateAgent = useCallback(
    async (editorId: string) => {
      const editor = editors.find((e) => e.editor_id === editorId)
      if (!editor) return
      setAgentConfigDialog({
        open: true,
        editorId: editor.editor_id,
        editorName: editor.name,
        configDir: '',
      })
    },
    [editors]
  )

  const handleAgentConfigSubmit = useCallback(async () => {
    const { editorId, editorName, configDir } = agentConfigDialog
    if (!configDir.trim()) {
      toast.error('Config directory path is required')
      return
    }
    try {
      await createAgent(fileId, configDir.trim(), editorName, editorId)
      setAgentConfigDialog((prev) => ({ ...prev, open: false }))
    } catch {
      // Error toast is handled by createAgent in app-store
    }
  }, [fileId, agentConfigDialog, createAgent])

  // Handler for toggling agent enable/disable status
  const handleToggleAgentStatus = useCallback(
    async (agentBlockId: string, currentStatus: string) => {
      try {
        if (currentStatus === 'enabled') {
          await disableAgent(fileId, agentBlockId)
        } else {
          await enableAgent(fileId, agentBlockId)
        }
      } catch (error) {
        console.error('Failed to toggle agent status:', error)
      }
    },
    [fileId, enableAgent, disableAgent]
  )

  // NOTE: We do NOT use deleteEditor here anymore.
  // "Removing access" simply means revoking all permissions on this block.
  // Deleting a user from the project entirely should be an admin/owner function in Sidebar.

  // Filter grants relevant to this block (including wildcards)
  const relevantGrants = useMemo(() => {
    return grants.filter((g) => g.block_id === blockId || g.block_id === '*')
  }, [grants, blockId])

  // Get editors with permissions for this block
  const collaborators = useMemo(() => {
    // Get all editors who have grants for this block, are the owner, or are the system editor
    const editorsWithAccess = editors.filter((editor) => {
      // Block owner (creator) always has access
      if (editor.editor_id === block.owner) return true

      // System editor (file owner) always has access — backend grants all permissions
      if (systemEditorId && editor.editor_id === systemEditorId) return true

      // Check if editor has any grants for this block (not wildcards for other blocks)
      return relevantGrants.some((g) => g.editor_id === editor.editor_id)
    })

    // Sort: system editor first, then block owner, then active editor, then others
    return editorsWithAccess.sort((a, b) => {
      // System editor (file owner) first
      if (systemEditorId && a.editor_id === systemEditorId) return -1
      if (systemEditorId && b.editor_id === systemEditorId) return 1

      // Block owner second
      if (a.editor_id === block.owner) return -1
      if (b.editor_id === block.owner) return 1

      // Active editor third
      if (a.editor_id === activeEditor?.editor_id) return -1
      if (b.editor_id === activeEditor?.editor_id) return 1

      // Others by name
      return a.name.localeCompare(b.name)
    })
  }, [editors, block, activeEditor, relevantGrants, systemEditorId])

  const handleGrantChange = async (
    editorId: string,
    capability: string,
    granted: boolean
  ) => {
    try {
      // Check if current user has permission to grant/revoke
      const requiredCap = granted ? 'core.grant' : 'core.revoke'
      const hasPermission = await checkPermission(fileId, blockId, requiredCap)

      if (!hasPermission) {
        toast.error(
          `You do not have permission to ${granted ? 'grant' : 'revoke'} permissions.`
        )
        return
      }

      // Check if this is a content read capability (markdown.read, code.read, directory.read)
      const isContentRead =
        capability.endsWith('.read') && capability !== 'core.read'

      if (granted) {
        // When granting content read, also grant core.read for metadata access
        if (isContentRead) {
          await grantCapability(fileId, editorId, 'core.read', blockId)
        }
        await grantCapability(fileId, editorId, capability, blockId)
      } else {
        // When revoking content read, also revoke core.read
        await revokeCapability(fileId, editorId, capability, blockId)
        if (isContentRead) {
          await revokeCapability(fileId, editorId, 'core.read', blockId)
        }
      }
    } catch (error) {
      console.error('Failed to change permission:', error)
    }
  }

  const handleRemoveAccess = async (editorId: string) => {
    // Revoke ALL permissions for this editor on this block
    try {
      const userGrants = relevantGrants.filter((g) => g.editor_id === editorId)

      // If user has no explicit grants (e.g. implicitly has access via wildcard or just viewing),
      // there's nothing to revoke, but we can't "block" them unless we have a deny list (not implemented).
      // For now, we just remove explicit grants.

      if (userGrants.length === 0) {
        toast.info('User has no explicit permissions to remove.')
        return
      }

      // Execute revokes in parallel
      await Promise.all(
        userGrants.map((grant) =>
          revokeCapability(fileId, editorId, grant.cap_id, blockId)
        )
      )

      toast.success('Access removed (permissions revoked)')
    } catch (error) {
      console.error('Failed to remove access:', error)
      toast.error('Failed to remove access')
    }
  }

  // Check permission state for granting (used to enable/disable Add button)
  const [canAddCollaborator, setCanAddCollaborator] = useState(false)

  useEffect(() => {
    const checkCanAddCollaborator = async () => {
      if (!activeEditor?.editor_id) {
        setCanAddCollaborator(false)
        return
      }
      try {
        // To add a collaborator, you generally need to be able to grant permissions
        // or create editors. We check 'core.grant' as the primary gatekeeper.
        const hasPermission = await checkPermission(
          fileId,
          blockId,
          'core.grant'
        )
        setCanAddCollaborator(hasPermission)
      } catch (error) {
        console.error('Failed to check permission:', error)
        setCanAddCollaborator(false)
      }
    }
    checkCanAddCollaborator()
  }, [fileId, blockId, activeEditor?.editor_id, checkPermission])

  const handleAddSuccess = (_editor: Editor) => {
    // Dialog handles the creation and granting. We just refresh the list (via store subscription).
    // No extra action needed here.
  }

  const handleOpenAddDialog = () => {
    setShowAddDialog(true)
  }

  // Show empty state if no collaborators (except maybe owner who is filtered out if logic changes, but currently owner is always shown)
  if (collaborators.length === 0) {
    return (
      <div className="space-y-3">
        {/* Header */}
        <div className="flex items-center justify-between px-1">
          <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
            Collaborators (0)
          </h3>
          <Button
            onClick={handleOpenAddDialog}
            variant="ghost"
            size="icon"
            className="h-6 w-6 hover:bg-muted"
            title={
              canAddCollaborator
                ? 'Add Collaborator'
                : 'You do not have permission to add collaborators'
            }
            disabled={!canAddCollaborator}
          >
            <UserPlus className="h-4 w-4" />
          </Button>
        </div>

        {/* Empty State */}
        <div className="flex flex-col items-center justify-center rounded-lg border border-dashed border-border/50 bg-muted/10 py-8 text-center">
          <Users className="mb-2 h-8 w-8 text-muted-foreground/50" />
          <p className="text-xs text-muted-foreground">No collaborators yet</p>
          {canAddCollaborator && (
            <Button
              variant="link"
              size="sm"
              onClick={handleOpenAddDialog}
              className="h-auto px-0 py-1 text-xs text-primary"
            >
              Add one now
            </Button>
          )}
        </div>

        <AddCollaboratorDialog
          fileId={fileId}
          blockId={blockId}
          blockType={block.block_type}
          existingEditors={collaborators}
          allEditors={editors}
          open={showAddDialog}
          onOpenChange={setShowAddDialog}
          onSuccess={handleAddSuccess}
        />
      </div>
    )
  }

  return (
    <div className="space-y-3">
      {/* Header */}
      <div className="flex items-center justify-between px-1">
        <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          Collaborators ({collaborators.length})
        </h3>
        <Button
          onClick={handleOpenAddDialog}
          variant="ghost"
          size="icon"
          className="h-6 w-6 hover:bg-muted"
          title={
            canAddCollaborator
              ? 'Add Collaborator'
              : 'You do not have permission to add collaborators'
          }
          disabled={!canAddCollaborator}
        >
          <UserPlus className="h-4 w-4" />
        </Button>
      </div>

      {/* Collaborator List */}
      <div className="space-y-2">
        {collaborators.map((editor) => (
          <CollaboratorItem
            key={editor.editor_id}
            blockId={blockId}
            blockType={block.block_type}
            editor={editor}
            grants={relevantGrants.filter(
              (g) => g.editor_id === editor.editor_id
            )}
            isOwner={editor.editor_id === block.owner}
            isFileOwner={
              systemEditorId != null && editor.editor_id === systemEditorId
            }
            isActive={editor.editor_id === activeEditor?.editor_id}
            isGlobal={isGlobalCollaborator(fileId, editor.editor_id)}
            onGrantChange={handleGrantChange}
            onRemoveAccess={handleRemoveAccess}
            agentBlock={findAgentBlockForEditor(editor)}
            onToggleAgentStatus={handleToggleAgentStatus}
            onCreateAgent={handleCreateAgent}
          />
        ))}
      </div>

      {/* Add Collaborator Dialog */}
      <AddCollaboratorDialog
        fileId={fileId}
        blockId={blockId}
        blockType={block.block_type}
        existingEditors={collaborators}
        allEditors={editors}
        open={showAddDialog}
        onOpenChange={setShowAddDialog}
        onSuccess={handleAddSuccess}
      />

      {/* Agent Config Dialog — prompts for config_dir when creating an agent */}
      <Dialog
        open={agentConfigDialog.open}
        onOpenChange={(open) =>
          setAgentConfigDialog((prev) => ({ ...prev, open }))
        }
      >
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Configure Agent</DialogTitle>
            <DialogDescription>
              Select the AI tool config directory for{' '}
              <strong>{agentConfigDialog.editorName}</strong> (e.g.{' '}
              <code>.claude</code> folder in your project).
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-3 py-2">
            <div className="space-y-2">
              <Label htmlFor="agent-config-dir">Config Directory</Label>
              <div className="flex gap-2">
                <Input
                  id="agent-config-dir"
                  placeholder="/path/to/project/.claude"
                  value={agentConfigDialog.configDir}
                  onChange={(e) =>
                    setAgentConfigDialog((prev) => ({
                      ...prev,
                      configDir: e.target.value,
                    }))
                  }
                  onKeyDown={(e) => {
                    if (
                      e.key === 'Enter' &&
                      agentConfigDialog.configDir.trim()
                    ) {
                      handleAgentConfigSubmit()
                    }
                  }}
                  className="flex-1"
                />
                <Button
                  variant="outline"
                  size="icon"
                  title="Browse..."
                  onClick={async () => {
                    try {
                      const selected = await open({
                        directory: true,
                        multiple: false,
                        title: `Select config directory for ${agentConfigDialog.editorName}`,
                      })
                      if (selected && typeof selected === 'string') {
                        setAgentConfigDialog((prev) => ({
                          ...prev,
                          configDir: selected,
                        }))
                      }
                    } catch {
                      // User cancelled the dialog
                    }
                  }}
                >
                  <FolderOpen className="h-4 w-4" />
                </Button>
              </div>
            </div>
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() =>
                setAgentConfigDialog((prev) => ({ ...prev, open: false }))
              }
            >
              Cancel
            </Button>
            <Button
              onClick={handleAgentConfigSubmit}
              disabled={!agentConfigDialog.configDir.trim()}
            >
              Create Agent
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
