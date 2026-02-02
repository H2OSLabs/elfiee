import { useState } from 'react'
import {
  User,
  Bot,
  Crown,
  Check,
  BookOpen,
  Edit2,
  Trash2,
  MoreVertical,
  Settings,
  UserMinus,
  Loader2,
  Sparkles,
} from 'lucide-react'
import { Checkbox } from '@/components/ui/checkbox'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Switch } from '@/components/ui/switch'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { ConfigureBotDialog } from './ConfigureBotDialog'
import type { AgentContents, Block, Editor, Grant } from '@/bindings'

interface CollaboratorItemProps {
  blockId: string
  blockType: string
  editor: Editor
  grants: Grant[]
  isOwner: boolean
  isActive: boolean
  onGrantChange: (
    editorId: string,
    capability: string,
    granted: boolean
  ) => Promise<void>
  onRemoveAccess?: (editorId: string) => Promise<void>
  agentBlock?: Block
  onToggleAgentStatus?: (
    agentBlockId: string,
    currentStatus: string
  ) => Promise<void>
  onCreateAgent?: (editorId: string) => Promise<void>
}

// Permission mapping:
// - All blocks have unified Read, Write, Delete permissions
// - Read: block_type.read (markdown.read, code.read, directory.read)
// - Write:
//   - Markdown/Code: block_type.write (markdown.write, code.write)
//   - Directory: Represents directory.create, delete, rename, export capabilities
// - Delete: core.delete (unified for all block types)
const getAvailableCapabilities = (blockType: string) => {
  if (blockType === 'code') {
    return [
      { id: 'code.read', label: 'Read', icon: BookOpen },
      { id: 'code.write', label: 'Write', icon: Edit2 },
      { id: 'core.delete', label: 'Delete', icon: Trash2 },
    ]
  } else if (blockType === 'directory') {
    return [
      { id: 'directory.read', label: 'Read', icon: BookOpen },
      // Directory Write represents directory.create, directory.delete, directory.rename, directory.export
      // Frontend shows as "Write" but backend checks these individual capabilities
      { id: 'directory.write', label: 'Write', icon: Edit2 },
      { id: 'core.delete', label: 'Delete', icon: Trash2 },
    ]
  } else {
    // markdown and other types default to markdown capabilities
    return [
      { id: 'markdown.read', label: 'Read', icon: BookOpen },
      { id: 'markdown.write', label: 'Write', icon: Edit2 },
      { id: 'core.delete', label: 'Delete', icon: Trash2 },
    ]
  }
}

export const CollaboratorItem = ({
  blockId,
  blockType,
  editor,
  grants,
  isOwner,
  isActive,
  onGrantChange,
  onRemoveAccess,
  agentBlock,
  onToggleAgentStatus,
  onCreateAgent,
}: CollaboratorItemProps) => {
  const [loadingCapabilities, setLoadingCapabilities] = useState<Set<string>>(
    new Set()
  )
  const [isRemoving, setIsRemoving] = useState(false)
  const [showConfigDialog, setShowConfigDialog] = useState(false)
  const [isTogglingAgent, setIsTogglingAgent] = useState(false)

  // Derive agent status from the associated agent block
  const agentContents = agentBlock?.contents as AgentContents | undefined
  const agentStatus = agentContents?.status
  const isAgentEnabled = agentStatus === 'enabled'

  // Check if editor has a specific capability for this block
  const hasCapability = (capabilityId: string): boolean => {
    // Owner always has all capabilities
    if (isOwner) return true

    // Check if there's a grant for this capability
    return grants.some(
      (g) =>
        g.editor_id === editor.editor_id &&
        g.cap_id === capabilityId &&
        (g.block_id === blockId || g.block_id === '*')
    )
  }

  const handleTogglePermission = async (capabilityId: string) => {
    // Owner permissions cannot be modified
    if (isOwner) return

    const currentlyHas = hasCapability(capabilityId)

    // Add to loading set
    setLoadingCapabilities((prev) => new Set(prev).add(capabilityId))

    try {
      await onGrantChange(editor.editor_id, capabilityId, !currentlyHas)
    } catch (error) {
      console.error('Failed to toggle permission:', error)
    } finally {
      // Remove from loading set
      setLoadingCapabilities((prev) => {
        const next = new Set(prev)
        next.delete(capabilityId)
        return next
      })
    }
  }

  const handleRemoveAccess = async () => {
    if (!onRemoveAccess) return

    setIsRemoving(true)
    try {
      await onRemoveAccess(editor.editor_id)
    } catch (error) {
      console.error('Failed to remove access:', error)
    } finally {
      setIsRemoving(false)
    }
  }

  const handleToggleAgent = async () => {
    setIsTogglingAgent(true)
    try {
      if (agentBlock && agentStatus && onToggleAgentStatus) {
        // Agent exists → toggle enable/disable
        await onToggleAgentStatus(agentBlock.block_id, agentStatus)
      } else if (!agentBlock && onCreateAgent) {
        // No agent yet → create one (auto-enabled)
        await onCreateAgent(editor.editor_id)
      }
    } catch (error) {
      console.error('Failed to toggle agent status:', error)
    } finally {
      setIsTogglingAgent(false)
    }
  }

  const isBot = editor.editor_type === 'Bot'

  // Get available capabilities for this block type
  const availableCapabilities = getAvailableCapabilities(blockType)

  return (
    <div className="rounded-lg border border-border/50 bg-card p-3 transition-colors hover:bg-muted/50">
      {/* Top Row: User Info */}
      <div className="mb-3 flex items-start justify-between">
        <div className="flex items-center gap-3">
          <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full border bg-background shadow-sm">
            {isOwner ? (
              <Crown className="h-4 w-4 text-amber-500" />
            ) : isBot ? (
              <Bot className="h-4 w-4 text-purple-500" />
            ) : (
              <User className="h-4 w-4 text-blue-500" />
            )}
          </div>

          <div className="flex flex-col gap-0.5">
            <div className="flex items-center gap-2">
              <span className="text-sm font-semibold text-foreground">
                {editor.name}
              </span>
              {isOwner && (
                <Badge
                  variant="secondary"
                  className="h-4 border-amber-200 bg-amber-100 px-1.5 text-[10px] text-amber-700 hover:bg-amber-100/80"
                >
                  Owner
                </Badge>
              )}
              {isActive && !isOwner && (
                <Badge
                  variant="outline"
                  className="h-4 border-green-200 bg-green-50 px-1.5 text-[10px] text-green-600"
                >
                  <Check className="mr-0.5 h-2.5 w-2.5" />
                  Active
                </Badge>
              )}
            </div>
            {isBot && (
              <span className="text-[10px] font-medium text-muted-foreground">
                AI Assistant
              </span>
            )}
          </div>
        </div>

        {!isOwner && (
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                className="h-6 w-6 text-muted-foreground hover:text-foreground"
                disabled={isRemoving}
                data-testid={`menu-trigger-${editor.editor_id}`}
              >
                <MoreVertical className="h-4 w-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              {isBot && (
                <DropdownMenuItem onSelect={() => setShowConfigDialog(true)}>
                  <Settings className="mr-2 h-4 w-4" />
                  <span>Configure</span>
                </DropdownMenuItem>
              )}
              <DropdownMenuItem
                className="text-destructive focus:text-destructive"
                onClick={handleRemoveAccess}
                disabled={isRemoving}
              >
                <UserMinus className="mr-2 h-4 w-4" />
                <span>{isRemoving ? 'Removing...' : 'Remove Access'}</span>
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        )}
      </div>

      {/* Agent Status Toggle - for all bot editors */}
      {isBot && (
        <div
          className={`mb-3 ml-12 flex items-center justify-between rounded-md border px-3 py-2 transition-all ${
            isAgentEnabled
              ? 'border-green-200 bg-green-50/50 dark:border-green-900/50 dark:bg-green-900/20'
              : 'border-border/50 bg-muted/20'
          }`}
        >
          <div className="flex items-center gap-2">
            <Sparkles
              className={`h-3.5 w-3.5 ${
                isAgentEnabled ? 'text-green-600' : 'text-muted-foreground'
              }`}
            />
            <span
              className={`text-xs font-medium ${
                isAgentEnabled
                  ? 'text-green-700 dark:text-green-400'
                  : 'text-muted-foreground'
              }`}
            >
              Agent Capabilities
            </span>
          </div>

          <div className="flex items-center gap-2">
            {isTogglingAgent ? (
              <div className="flex items-center gap-1.5">
                <Loader2 className="h-3 w-3 animate-spin text-muted-foreground" />
                <span className="text-[10px] text-muted-foreground">
                  Updating...
                </span>
              </div>
            ) : (
              <span
                className={`text-[10px] font-medium transition-colors ${
                  isAgentEnabled
                    ? 'text-green-600 dark:text-green-400'
                    : 'text-muted-foreground'
                }`}
              >
                {isAgentEnabled ? 'Active' : 'Inactive'}
              </span>
            )}

            <Switch
              checked={isAgentEnabled}
              onCheckedChange={handleToggleAgent}
              disabled={isTogglingAgent}
              className="h-4 w-7 data-[state=checked]:bg-green-500"
              data-testid={`agent-toggle-${editor.editor_id}`}
            />
          </div>
        </div>
      )}

      {/* Bottom Row: Permissions */}
      <div className="flex flex-wrap items-center gap-3 pl-12">
        {availableCapabilities.map((capability) => {
          const Icon = capability.icon
          const checked = hasCapability(capability.id)
          const isLoading = loadingCapabilities.has(capability.id)

          return (
            <div
              key={capability.id}
              onClick={() =>
                !isOwner && !isLoading && handleTogglePermission(capability.id)
              }
              className={`group flex items-center gap-2 rounded-md px-2.5 py-1.5 transition-all ${
                isOwner
                  ? 'cursor-not-allowed bg-muted/50'
                  : isLoading
                    ? 'cursor-wait bg-muted/50'
                    : checked
                      ? 'cursor-pointer bg-primary/10 hover:bg-primary/20'
                      : 'cursor-pointer bg-muted/30 hover:bg-muted/50'
              } `}
              title={
                isOwner
                  ? 'Owner has all permissions'
                  : isLoading
                    ? 'Updating permission...'
                    : checked
                      ? `Click to revoke ${capability.label} permission`
                      : `Click to grant ${capability.label} permission`
              }
            >
              <Checkbox
                id={`${editor.editor_id}-${capability.id}`}
                checked={checked}
                disabled={isOwner || isLoading}
                className="pointer-events-none h-4 w-4"
                data-testid={`checkbox-${editor.editor_id}-${capability.id}`}
              />
              <div
                className={`flex items-center gap-1.5 text-xs font-medium transition-colors ${
                  isOwner
                    ? 'text-muted-foreground'
                    : isLoading
                      ? 'text-muted-foreground'
                      : checked
                        ? 'text-primary group-hover:text-primary'
                        : 'text-foreground group-hover:text-primary'
                }`}
              >
                <Icon className="h-3.5 w-3.5" />
                <span>{capability.label}</span>
              </div>
            </div>
          )
        })}
      </div>

      <ConfigureBotDialog
        open={showConfigDialog}
        onOpenChange={setShowConfigDialog}
        botName={editor.name}
        onSave={async (config) => {
          console.log('Saving config:', config)
          // TODO: Implement actual save logic
          return Promise.resolve()
        }}
      />
    </div>
  )
}
