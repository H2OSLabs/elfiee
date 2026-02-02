import { useMemo, useState, useRef, useEffect } from 'react'
import {
  Calendar,
  Clock,
  User,
  FileText,
  Tag,
  Edit2,
  RotateCcw,
  PlusCircle,
  Trash2,
  ShieldCheck,
  ShieldAlert,
  Activity,
  History,
  Copy,
  Check,
  Link2,
  Unlink,
  Plus,
} from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import { Separator } from '@/components/ui/separator'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import { useAppStore } from '@/lib/app-store'
import { sortEventsByVectorClock } from '@/utils/event-utils'
import { CollaboratorList } from '@/components/permission/CollaboratorList'
import { toast } from 'sonner'
import { ConfirmDeleteDialog } from './ConfirmDeleteDialog'
import type { Block, Event } from '@/bindings'

const InfoTab = ({
  block,
  fileId,
}: {
  block: Block | null
  fileId: string | null
}) => {
  const [isEditingDescription, setIsEditingDescription] = useState(false)
  const [editedDescription, setEditedDescription] = useState('')
  const [isSaving, setIsSaving] = useState(false)
  const [copiedBlockId, setCopiedBlockId] = useState(false)
  const textareaRef = useRef<HTMLTextAreaElement>(null)
  const { updateBlockMetadata, getEditors, checkPermission } = useAppStore()

  // Extract metadata
  const metadata = block?.metadata as {
    description?: string
    created_at?: string
    updated_at?: string
  } | null
  const description = metadata?.description || ''
  const createdAt = metadata?.created_at
  const updatedAt = metadata?.updated_at

  const formatDate = (isoString?: string) => {
    if (!isoString) return '-'
    try {
      return new Date(isoString).toLocaleDateString('zh-CN', {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit',
      })
    } catch {
      return '-'
    }
  }

  // Resolve owner info
  const ownerEditor =
    fileId && block
      ? getEditors(fileId).find((e) => e.editor_id === block.owner)
      : null
  const ownerName = ownerEditor?.name || block?.owner || ''
  const ownerIdPreview = block?.owner.slice(0, 8) || ''

  // Copy block ID to clipboard
  const handleCopyBlockId = async () => {
    if (!block?.block_id) return

    try {
      await navigator.clipboard.writeText(block.block_id)
      setCopiedBlockId(true)
      toast.success('Block ID copied to clipboard')
      setTimeout(() => setCopiedBlockId(false), 2000)
    } catch (error) {
      toast.error('Failed to copy Block ID')
    }
  }

  // Start editing description
  const handleStartEdit = () => {
    setEditedDescription(description)
    setIsEditingDescription(true)
  }

  // Cancel editing
  const handleCancelEdit = () => {
    setIsEditingDescription(false)
    setEditedDescription('')
  }

  // Save description
  const handleSaveDescription = async () => {
    if (!fileId || !block) return

    setIsSaving(true)
    try {
      // Permission check using Action
      const hasPermission = await checkPermission(
        fileId,
        block.block_id,
        'core.update_metadata'
      )

      if (!hasPermission) {
        toast.error('You do not have permission to update metadata.')
        return
      }

      await updateBlockMetadata(fileId, block.block_id, {
        description: editedDescription,
      })
      setIsEditingDescription(false)
    } catch (error) {
      console.error('Failed to save description:', error)
    } finally {
      setIsSaving(false)
    }
  }

  useEffect(() => {
    if (isEditingDescription && textareaRef.current) {
      const textarea = textareaRef.current
      textarea.focus()
      textarea.style.height = 'auto'
      textarea.style.height = textarea.scrollHeight + 'px'
    }
  }, [isEditingDescription])

  if (!block) {
    return (
      <div className="flex flex-col items-center justify-center py-12 text-center text-muted-foreground">
        <FileText className="mb-3 h-10 w-10 opacity-20" />
        <p className="text-sm font-medium">No block selected</p>
        <p className="text-xs opacity-70">Select a block to view its details</p>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      {/* Header Section */}
      <div className="space-y-2">
        <div className="flex items-start justify-between gap-2">
          <h2 className="text-lg font-semibold leading-tight tracking-tight text-foreground">
            {block.name}
          </h2>
          <Badge
            variant="secondary"
            className="shrink-0 font-mono text-[10px] uppercase"
          >
            {block.block_type}
          </Badge>
        </div>
      </div>

      <Separator />

      {/* Description Section */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <h3 className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
            <FileText className="h-3.5 w-3.5" />
            Description
          </h3>
          {!isEditingDescription && (
            <Button
              variant="ghost"
              size="icon"
              className="h-6 w-6 text-muted-foreground hover:text-foreground"
              onClick={handleStartEdit}
            >
              <Edit2 className="h-3.5 w-3.5" />
            </Button>
          )}
        </div>

        {isEditingDescription ? (
          <div className="space-y-2">
            <Textarea
              ref={textareaRef}
              value={editedDescription}
              onChange={(e) => {
                setEditedDescription(e.target.value)
                e.target.style.height = 'auto'
                e.target.style.height = e.target.scrollHeight + 'px'
              }}
              className="min-h-[80px] resize-none text-sm"
              placeholder="Add a description..."
            />
            <div className="flex justify-end gap-2">
              <Button
                size="sm"
                variant="ghost"
                className="h-7 px-3 text-xs"
                onClick={handleCancelEdit}
              >
                Cancel
              </Button>
              <Button
                size="sm"
                className="h-7 px-3 text-xs"
                onClick={handleSaveDescription}
                disabled={isSaving}
              >
                {isSaving ? 'Saving...' : 'Save'}
              </Button>
            </div>
          </div>
        ) : (
          <div className="rounded-md bg-muted/20 p-3 text-sm leading-relaxed text-foreground">
            {description ? (
              description
            ) : (
              <span className="italic text-muted-foreground/60">
                No description provided.
              </span>
            )}
          </div>
        )}
      </div>

      <Separator />

      {/* Details Grid */}
      <div className="grid gap-4">
        <h3 className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
          <Tag className="h-3.5 w-3.5" />
          Details
        </h3>

        <div className="overflow-hidden rounded-lg border border-border/50 bg-background text-sm shadow-sm">
          {/* Owner Row */}
          <div className="flex items-center justify-between border-b border-border/50 bg-muted/20 px-4 py-3">
            <div className="flex items-center gap-2 text-muted-foreground">
              <User className="h-3.5 w-3.5" />
              <span>Owner</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="h-2 w-2 rounded-full bg-green-500 shadow-[0_0_8px_rgba(34,197,94,0.3)]" />
              <span className="font-semibold text-foreground">
                {ownerName}{' '}
                <span className="text-xs text-muted-foreground">
                  ({ownerIdPreview}...)
                </span>
              </span>
            </div>
          </div>

          <div className="grid gap-y-3 p-4">
            {/* Block ID Row with Copy Button */}
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2 text-muted-foreground">
                <Tag className="h-3.5 w-3.5 opacity-70" />
                <span>Block ID</span>
              </div>
              <div className="flex items-center gap-2">
                <span
                  className="max-w-[140px] truncate font-mono text-xs text-muted-foreground"
                  title={block.block_id}
                >
                  {block.block_id}
                </span>
                <Button
                  size="icon"
                  variant="ghost"
                  className="h-5 w-5 text-muted-foreground hover:text-foreground"
                  onClick={handleCopyBlockId}
                  title="Copy Block ID"
                >
                  {copiedBlockId ? (
                    <Check className="h-3 w-3 text-green-500" />
                  ) : (
                    <Copy className="h-3 w-3" />
                  )}
                </Button>
              </div>
            </div>

            {/* Created - Always show */}
            {createdAt && (
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2 text-muted-foreground">
                  <Calendar className="h-3.5 w-3.5 opacity-70" />
                  <span>Created</span>
                </div>
                <span className="font-mono text-xs text-muted-foreground">
                  {formatDate(createdAt)}
                </span>
              </div>
            )}

            {updatedAt && (
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2 text-muted-foreground">
                  <Clock className="h-3.5 w-3.5 opacity-70" />
                  <span>Updated</span>
                </div>
                <span className="font-mono text-xs text-muted-foreground">
                  {formatDate(updatedAt)}
                </span>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}

const CollaboratorsTab = ({
  block,
  fileId,
}: {
  block: Block | null
  fileId: string | null
}) => {
  if (!block || !fileId) {
    return (
      <div className="flex h-32 items-center justify-center rounded-lg border border-dashed border-border/50 text-center">
        <div>
          <p className="text-sm text-muted-foreground">No block selected</p>
          <p className="mt-1 text-xs text-muted-foreground/70">
            Select a block to manage collaborators
          </p>
        </div>
      </div>
    )
  }

  return (
    <CollaboratorList fileId={fileId} blockId={block.block_id} block={block} />
  )
}

const TimelineTab = ({
  events,
  fileId,
  blockId,
}: {
  events: Event[]
  fileId: string | null
  blockId: string | null
}) => {
  const { restoreToEvent } = useAppStore()
  const [isRestoring, setIsRestoring] = useState(false)

  // Retrieve editors to resolve names
  const editors = useAppStore((state) => {
    if (!fileId) return []
    return state.files.get(fileId)?.editors || []
  })

  const sortedEvents = useMemo(() => {
    // Use the standardized vector clock sorting for ordering (descending: newest first)
    return sortEventsByVectorClock(events)
  }, [events])

  const handleRestore = async (eventId: string) => {
    if (!fileId || !blockId) return
    setIsRestoring(true)
    try {
      await restoreToEvent(fileId, blockId, eventId)
    } catch (error) {
      console.error('Failed to restore:', error)
    } finally {
      setIsRestoring(false)
    }
  }

  const getEventMeta = (capId: string, value?: any) => {
    if (capId.includes('create')) {
      return {
        icon: PlusCircle,
        color: 'text-emerald-500',
        bg: 'bg-emerald-500/10',
        label: 'Created',
        desc: 'Created new block',
      }
    }
    if (capId.includes('write') || capId.includes('update')) {
      return {
        icon: Edit2,
        color: 'text-blue-500',
        bg: 'bg-blue-500/10',
        label: 'Edited',
        desc: 'Modified content',
      }
    }
    if (capId.includes('delete')) {
      return {
        icon: Trash2,
        color: 'text-red-500',
        bg: 'bg-red-500/10',
        label: 'Deleted',
        desc: 'Removed block',
      }
    }
    if (capId.includes('grant')) {
      let desc = 'Granted permission'
      if (value && value.editor && value.capability) {
        const targetEditor = editors.find((e) => e.editor_id === value.editor)
        const targetName = targetEditor ? targetEditor.name : value.editor
        desc = `Granted '${value.capability}' to ${targetName}`
      }
      return {
        icon: ShieldCheck,
        color: 'text-indigo-500',
        bg: 'bg-indigo-500/10',
        label: 'Access',
        desc,
      }
    }
    if (capId.includes('revoke')) {
      let desc = 'Revoked permission'
      if (value && value.editor && value.capability) {
        const targetEditor = editors.find((e) => e.editor_id === value.editor)
        const targetName = targetEditor ? targetEditor.name : value.editor
        desc = `Revoked '${value.capability}' from ${targetName}`
      }
      return {
        icon: ShieldAlert,
        color: 'text-orange-500',
        bg: 'bg-orange-500/10',
        label: 'Access',
        desc,
      }
    }
    return {
      icon: Activity,
      color: 'text-foreground',
      bg: 'bg-muted',
      label: 'System',
      desc: 'System activity',
    }
  }

  if (sortedEvents.length === 0) {
    return (
      <div className="flex h-40 flex-col items-center justify-center p-4 text-center">
        <div className="mb-3 rounded-full bg-muted/50 p-3">
          <History className="h-5 w-5 text-muted-foreground/50" />
        </div>
        <p className="text-sm font-medium text-foreground">No History</p>
        <p className="mt-1 text-xs text-muted-foreground">
          Changes will appear here
        </p>
      </div>
    )
  }

  return (
    <div className="relative px-4 pb-4 pt-2">
      <div className="ml-2.5 space-y-6 border-l border-border/50 pl-6 pt-1">
        {sortedEvents.map((event) => {
          const [editorId, capId] = event.attribute.split('/')

          // Resolve editor name
          let operatorName = editorId
          if (editorId === 'system') {
            operatorName = 'System'
          } else {
            const editor = editors.find((e) => e.editor_id === editorId)
            if (editor) {
              operatorName = editor.name
            }
          }

          const meta = getEventMeta(capId, event.value)
          const Icon = meta.icon
          const date = new Date(event.created_at)

          return (
            <div key={event.event_id} className="group relative">
              {/* Icon Marker */}
              <div
                className={`absolute -left-[31px] top-0.5 flex h-5 w-5 items-center justify-center rounded-full border bg-background ring-4 ring-background transition-colors ${meta.color} border-border/50`}
              >
                <Icon className="h-2.5 w-2.5" />
              </div>

              {/* Content */}
              <div className="flex flex-col gap-1.5">
                <div className="flex items-center justify-between gap-4">
                  <div className="flex flex-col gap-0.5">
                    <span className="text-xs font-semibold text-foreground/90">
                      {operatorName}
                    </span>
                    <span className="text-[10px] text-muted-foreground">
                      {date.toLocaleTimeString([], {
                        hour: '2-digit',
                        minute: '2-digit',
                      })}{' '}
                      · {date.toLocaleDateString()}
                    </span>
                  </div>
                  <Badge
                    variant="secondary"
                    className={`h-5 px-1.5 text-[10px] font-normal tracking-wide ${meta.bg} ${meta.color} hover:${meta.bg} border-0`}
                  >
                    {meta.label}
                  </Badge>
                </div>

                <div className="relative rounded-md border border-border/40 bg-muted/20 p-2.5 transition-all hover:bg-muted/40 group-hover:border-border/60">
                  <p className="text-xs leading-relaxed text-foreground/80">
                    {meta.desc}
                  </p>

                  {/* Only show Restore button for content-modifying events */}
                  {(capId.includes('write') ||
                    capId.includes('update') ||
                    capId.includes('create')) && (
                    <div className="absolute right-2 top-2 opacity-0 transition-opacity group-hover:opacity-100">
                      <Button
                        size="icon"
                        variant="ghost"
                        className="h-6 w-6 text-muted-foreground hover:text-foreground"
                        onClick={() => handleRestore(event.event_id)}
                        disabled={isRestoring}
                        title="Restore this version"
                      >
                        <RotateCcw className="h-3 w-3" />
                      </Button>
                    </div>
                  )}
                </div>
              </div>
            </div>
          )
        })}
      </div>
    </div>
  )
}

const LinksTab = ({
  block,
  fileId,
}: {
  block: Block | null
  fileId: string | null
}) => {
  const { getBlocks, linkBlock, unlinkBlock } = useAppStore()
  const [isAdding, setIsAdding] = useState(false)

  const allBlocks = useMemo(
    () => (fileId ? getBlocks(fileId) : []),
    [fileId, getBlocks]
  )

  // Get linked block IDs from children.implement
  const linkedIds = useMemo(() => {
    if (!block?.children) return []
    const children = block.children as Partial<Record<string, string[]>>
    return children['implement'] || []
  }, [block?.children])

  // Resolve linked blocks
  const linkedBlocks = useMemo(
    () =>
      linkedIds
        .map((id) => allBlocks.find((b) => b.block_id === id))
        .filter((b): b is Block => !!b),
    [linkedIds, allBlocks]
  )

  // Available blocks for linking (exclude self and already linked)
  const availableBlocks = useMemo(() => {
    if (!block) return []
    const excluded = new Set([block.block_id, ...linkedIds])
    return allBlocks.filter(
      (b) => !excluded.has(b.block_id) && b.block_type !== 'directory' // directories are not linkable targets
    )
  }, [block, linkedIds, allBlocks])

  const handleLink = async (targetId: string) => {
    if (!fileId || !block) return
    try {
      await linkBlock(fileId, block.block_id, targetId)
      setIsAdding(false)
    } catch {
      // Error toast already shown
    }
  }

  const handleUnlink = async (targetId: string) => {
    if (!fileId || !block) return
    try {
      await unlinkBlock(fileId, block.block_id, targetId)
    } catch {
      // Error toast already shown
    }
  }

  if (!block || !fileId) {
    return (
      <div className="flex h-32 items-center justify-center rounded-lg border border-dashed border-border/50 text-center">
        <div>
          <p className="text-sm text-muted-foreground">No block selected</p>
          <p className="mt-1 text-xs text-muted-foreground/70">
            Select a block to manage links
          </p>
        </div>
      </div>
    )
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
          <Link2 className="h-3.5 w-3.5" />
          Implement Relations
        </h3>
        <Button
          variant="ghost"
          size="icon"
          className="h-6 w-6 text-muted-foreground hover:text-foreground"
          onClick={() => setIsAdding(!isAdding)}
        >
          <Plus className="h-3.5 w-3.5" />
        </Button>
      </div>

      {/* Add link selector */}
      {isAdding && (
        <div className="space-y-2 rounded-md border border-border/50 bg-muted/20 p-3">
          <p className="text-xs font-medium text-muted-foreground">
            Select a block to link:
          </p>
          <div className="max-h-[200px] space-y-1 overflow-y-auto">
            {availableBlocks.length === 0 ? (
              <p className="py-2 text-xs text-muted-foreground/60">
                No available blocks to link
              </p>
            ) : (
              availableBlocks.map((b) => (
                <button
                  key={b.block_id}
                  className="flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-left text-sm transition-colors hover:bg-muted"
                  onClick={() => handleLink(b.block_id)}
                >
                  <Badge
                    variant="secondary"
                    className="shrink-0 font-mono text-[9px] uppercase"
                  >
                    {b.block_type}
                  </Badge>
                  <span className="truncate">{b.name}</span>
                </button>
              ))
            )}
          </div>
          <Button
            variant="ghost"
            size="sm"
            className="h-7 w-full text-xs"
            onClick={() => setIsAdding(false)}
          >
            Cancel
          </Button>
        </div>
      )}

      {/* Linked blocks list */}
      {linkedBlocks.length === 0 && !isAdding ? (
        <div className="flex flex-col items-center justify-center rounded-lg border border-dashed border-border/50 py-8 text-center">
          <Link2 className="mb-2 h-6 w-6 text-muted-foreground/30" />
          <p className="text-sm text-muted-foreground">No linked blocks</p>
          <p className="mt-1 text-xs text-muted-foreground/70">
            Link blocks to create implement relations
          </p>
        </div>
      ) : (
        <div className="space-y-2">
          {linkedBlocks.map((linked) => (
            <div
              key={linked.block_id}
              className="group flex items-center justify-between rounded-md border border-border/40 bg-muted/20 px-3 py-2 transition-colors hover:bg-muted/40"
            >
              <div className="flex min-w-0 items-center gap-2">
                <Badge
                  variant="secondary"
                  className="shrink-0 font-mono text-[9px] uppercase"
                >
                  {linked.block_type}
                </Badge>
                <span className="truncate text-sm text-foreground">
                  {linked.name}
                </span>
              </div>
              <Button
                variant="ghost"
                size="icon"
                className="h-6 w-6 shrink-0 text-muted-foreground opacity-0 hover:text-destructive group-hover:opacity-100"
                onClick={() => handleUnlink(linked.block_id)}
                title="Remove link"
              >
                <Unlink className="h-3 w-3" />
              </Button>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

const ActiveTerminalsList = ({ documentId }: { documentId: string }) => {
  const blocks = useAppStore((state) => {
    const fileState = state.files.get(documentId)
    return fileState?.blocks || []
  })
  const selectBlock = useAppStore((state) => state.selectBlock)
  const selectedBlockId = useAppStore((state) => state.selectedBlockId)
  const deleteBlock = useAppStore((state) => state.deleteBlock)
  const closeTerminalSession = useAppStore(
    (state) => state.closeTerminalSession
  )
  const activeEditorId = useAppStore((state) => {
    const fileState = state.files.get(documentId)
    return fileState?.activeEditorId
  })

  // useState must be called before any conditional returns (React Hooks rules)
  const [blockToDelete, setBlockToDelete] = useState<Block | null>(null)

  // Filter ONLY 'terminal' type blocks
  // Reverse to show newest first (assuming append order)
  const terminalBlocks = blocks
    .filter((b) => b.block_type === 'terminal')
    .reverse()

  // Show nothing if no terminal blocks exist
  if (terminalBlocks.length === 0) return null

  const handleCopyId = (e: React.MouseEvent, id: string) => {
    e.stopPropagation()
    navigator.clipboard.writeText(id)
    toast.success('Terminal ID copied')
  }

  const handleDeleteClick = (e: React.MouseEvent, block: Block) => {
    e.stopPropagation()
    setBlockToDelete(block)
  }

  const confirmDelete = async () => {
    if (!blockToDelete) return

    try {
      // 1. Try to close the PTY session first (if it's active)
      if (activeEditorId) {
        closeTerminalSession(
          documentId,
          blockToDelete.block_id,
          activeEditorId
        ).catch(() => {})
      }

      // 2. Delete the block from the file (handles selection clear in store)
      await deleteBlock(documentId, blockToDelete.block_id)
    } catch (error) {
      console.error('Failed to delete terminal block:', error)
    } finally {
      setBlockToDelete(null)
    }
  }

  return (
    <div className="flex shrink-0 flex-col border-t border-border bg-card">
      <div className="flex items-center justify-between border-b border-border/50 bg-muted/10 px-4 py-2">
        <h3 className="text-[10px] font-bold uppercase tracking-wider text-muted-foreground">
          Terminals ({terminalBlocks.length})
        </h3>
      </div>
      <ScrollArea className="h-auto max-h-[180px] w-full">
        <div className="flex flex-col gap-1 p-2">
          {terminalBlocks.map((block) => {
            const isSelected = block.block_id === selectedBlockId
            return (
              <div
                key={block.block_id}
                className={`group flex cursor-pointer items-center justify-between rounded px-2 py-1.5 text-sm transition-all ${
                  isSelected
                    ? 'bg-accent text-accent-foreground ring-1 ring-inset ring-primary/20'
                    : 'hover:bg-accent/50 hover:text-accent-foreground'
                }`}
                onClick={() => selectBlock(block.block_id)}
              >
                <div className="flex items-center gap-2 overflow-hidden">
                  <div
                    className={`flex h-1.5 w-1.5 shrink-0 rounded-full shadow-[0_0_4px_rgba(34,197,94,0.4)] ${
                      isSelected
                        ? 'bg-green-500 ring-2 ring-green-500/20'
                        : 'bg-green-500/70'
                    }`}
                  />
                  <span
                    className={`truncate font-medium ${
                      isSelected
                        ? 'text-accent-foreground'
                        : 'text-foreground/80 group-hover:text-accent-foreground'
                    }`}
                  >
                    {block.name}
                  </span>
                </div>
                <div className="flex items-center gap-2 opacity-0 transition-opacity group-hover:opacity-100">
                  <span
                    className={`cursor-copy font-mono text-[10px] transition-colors ${
                      isSelected
                        ? 'text-accent-foreground/70'
                        : 'text-muted-foreground/60 hover:text-foreground'
                    }`}
                    onClick={(e) => handleCopyId(e, block.block_id)}
                    title="Click to copy ID"
                  >
                    {block.block_id.slice(0, 6)}
                  </span>
                  <Button
                    size="icon"
                    variant="ghost"
                    className="h-6 w-6 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
                    onClick={(e) => handleDeleteClick(e, block)}
                    title="Delete Terminal"
                  >
                    <Trash2 className="h-3 w-3" />
                  </Button>
                </div>
              </div>
            )
          })}
        </div>
      </ScrollArea>

      <ConfirmDeleteDialog
        isOpen={!!blockToDelete}
        onClose={() => setBlockToDelete(null)}
        onConfirm={confirmDelete}
        title={`Delete Terminal Session?`}
        description="Are you sure you want to permanently delete this terminal? This action cannot be undone."
        details={
          <>
            <span className="text-sm font-semibold text-foreground">
              {blockToDelete?.name}
            </span>
            <span className="truncate font-mono text-[10px] text-muted-foreground">
              ID: {blockToDelete?.block_id}
            </span>
          </>
        }
      />
    </div>
  )
}

const ContextPanel = () => {
  // CRITICAL: Use selector properly - all dependencies from 'state' parameter
  const block: Block | null = useAppStore((state) => {
    const currentFileId = state.currentFileId
    const selectedBlockId = state.selectedBlockId

    if (!currentFileId || !selectedBlockId) return null
    const fileState = state.files.get(currentFileId)
    return fileState?.blocks.find((b) => b.block_id === selectedBlockId) || null
  })

  // Get values separately for use in other components and hooks
  const currentFileId = useAppStore((state) => state.currentFileId)
  const selectedBlockId = useAppStore((state) => state.selectedBlockId)

  // IMPORTANT: Permission checks are handled by backend
  // Backend only returns blocks/data that the current user has permission to access
  // Frontend simply renders what backend provides

  // 1. Select the raw events array (stable reference from store)
  const allEvents = useAppStore((state) => {
    if (!currentFileId) return []
    return state.files.get(currentFileId)?.events || []
  })

  // 2. Filter events locally to avoid new array references in the selector
  const events = useMemo(() => {
    if (!block) return allEvents
    return allEvents.filter((e) => {
      // Block events: entity matches block_id
      if (e.entity === block.block_id) return true

      // Grant/Revoke events: check if the event value contains this block_id
      // Grant/revoke events have entity as editor_id, but value.block contains the target block
      if (
        e.attribute.includes('core.grant') ||
        e.attribute.includes('core.revoke')
      ) {
        const value = e.value as { block?: string } | null
        if (value?.block === block.block_id || value?.block === '*') {
          return true
        }
      }

      return false
    })
  }, [allEvents, block?.block_id])

  if (!currentFileId) {
    return (
      <aside className="flex h-full w-full items-center justify-center bg-card text-muted-foreground">
        <p className="text-sm">No file opened</p>
      </aside>
    )
  }

  return (
    <aside className="flex h-full w-full flex-col bg-card">
      <div className="shrink-0 border-b border-border px-4 pb-2 pt-4">
        <div className="mb-3 flex items-center gap-2">
          <Badge
            variant="outline"
            className="h-5 border-border px-2 text-[10px] uppercase tracking-wide text-muted-foreground"
          >
            Scope
          </Badge>
          <span className="text-sm font-medium text-foreground">
            {block ? block.name : 'Blocks'}
          </span>
        </div>
      </div>

      <Tabs defaultValue="info" className="flex min-h-0 flex-1 flex-col">
        <div className="shrink-0 border-b border-border px-4">
          <TabsList className="h-auto w-full justify-start gap-4 bg-transparent p-0">
            <TabsTrigger
              value="info"
              className="rounded-none px-1 pb-3 text-sm text-muted-foreground data-[state=active]:border-b-2 data-[state=active]:border-foreground data-[state=active]:bg-transparent data-[state=active]:text-foreground data-[state=active]:shadow-none"
            >
              Info
            </TabsTrigger>
            <TabsTrigger
              value="collaborators"
              className="rounded-none px-1 pb-3 text-sm text-muted-foreground data-[state=active]:border-b-2 data-[state=active]:border-foreground data-[state=active]:bg-transparent data-[state=active]:text-foreground data-[state=active]:shadow-none"
            >
              Collaborators
            </TabsTrigger>
            <TabsTrigger
              value="links"
              className="rounded-none px-1 pb-3 text-sm text-muted-foreground data-[state=active]:border-b-2 data-[state=active]:border-foreground data-[state=active]:bg-transparent data-[state=active]:text-foreground data-[state=active]:shadow-none"
            >
              Links
            </TabsTrigger>
            <TabsTrigger
              value="timeline"
              className="rounded-none px-1 pb-3 text-sm text-muted-foreground data-[state=active]:border-b-2 data-[state=active]:border-foreground data-[state=active]:bg-transparent data-[state=active]:text-foreground data-[state=active]:shadow-none"
            >
              Timeline
            </TabsTrigger>
          </TabsList>
        </div>

        <ScrollArea className="min-h-0 flex-1">
          <div className="space-y-6 p-4">
            <TabsContent value="info" className="mt-0">
              <InfoTab block={block} fileId={currentFileId} />
            </TabsContent>

            <TabsContent value="collaborators" className="mt-0">
              <CollaboratorsTab block={block} fileId={currentFileId} />
            </TabsContent>

            <TabsContent value="links" className="mt-0">
              <LinksTab block={block} fileId={currentFileId} />
            </TabsContent>

            <TabsContent value="timeline" className="mt-0">
              <TimelineTab
                events={events}
                fileId={currentFileId}
                blockId={selectedBlockId}
              />
            </TabsContent>
          </div>
        </ScrollArea>
      </Tabs>

      {/* Active Terminals List - Always visible at the bottom */}
      <ActiveTerminalsList documentId={currentFileId} />
    </aside>
  )
}

export default ContextPanel
