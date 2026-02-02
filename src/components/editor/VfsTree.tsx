import { useState, useRef, useEffect } from 'react'
import {
  ChevronDown,
  ChevronRight,
  Folder,
  FileText,
  FileCode,
  CheckSquare,
  Plus,
  MoreHorizontal,
  Trash2,
  Pencil,
  Download,
} from 'lucide-react'
import { cn } from '@/lib/utils'
import type { VfsNode } from '@/utils/vfs-tree'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'

// --- Inline Edit Input ---
interface InlineEditInputProps {
  initialValue: string
  onSave: (value: string) => void
  onCancel: () => void
  depth: number
  type: 'file' | 'directory'
  blockType?: string
}

const InlineEditInput = ({
  initialValue,
  onSave,
  onCancel,
  depth,
  type,
  blockType,
}: InlineEditInputProps) => {
  const [value, setValue] = useState(initialValue)
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    inputRef.current?.focus()
    inputRef.current?.select()
  }, [])

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault()
      onSave(value.trim())
    } else if (e.key === 'Escape') {
      e.preventDefault()
      onCancel()
    }
  }

  const handleBlur = () => {
    onSave(value.trim())
  }

  const Icon =
    type === 'directory'
      ? Folder
      : blockType === 'task'
        ? CheckSquare
        : blockType === 'markdown'
          ? FileText
          : FileCode

  return (
    <div
      className="group flex w-full items-center overflow-hidden rounded-md bg-accent/10 py-1 pr-2 ring-1 ring-accent/50"
      style={{ paddingLeft: `${8 + depth * 16}px` }}
    >
      <div className="h-5 w-5 flex-shrink-0" />
      <Icon className="h-4 w-4 flex-shrink-0 text-accent" />
      <div className="ml-2 min-w-0 flex-1">
        <input
          ref={inputRef}
          type="text"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={handleKeyDown}
          onBlur={handleBlur}
          className="h-6 w-full border-none bg-transparent p-0 text-sm font-medium text-foreground focus:outline-none focus:ring-0"
        />
      </div>
    </div>
  )
}

// --- Single Tree Node ---
interface TreeNodeProps {
  node: VfsNode
  depth: number
  activeBlockId: string | null
  editingPath: string | null
  expandedPaths: Set<string>
  onSelect: (node: VfsNode) => void
  onToggle: (path: string) => void
  onAddChild: (node: VfsNode, type: 'file' | 'directory') => void
  onRename: (node: VfsNode, newName: string) => void
  onDelete: (node: VfsNode) => void
  onExport: (node: VfsNode) => void
  onStartRename: (path: string) => void
  onCancelRename: () => void
}

const TreeNode = ({
  node,
  depth,
  activeBlockId,
  editingPath,
  expandedPaths,
  onSelect,
  onToggle,
  onAddChild,
  onRename,
  onDelete,
  onExport,
  onStartRename,
  onCancelRename,
}: TreeNodeProps) => {
  const isSelected = node.blockId ? activeBlockId === node.blockId : false
  const isEditing = editingPath === node.path
  const isExpanded = expandedPaths.has(node.path)

  const handleToggle = (e: React.MouseEvent) => {
    e.stopPropagation()
    onToggle(node.path)
  }

  const handleSelect = (e: React.MouseEvent) => {
    e.stopPropagation()
    onSelect(node)
  }

  const handleAddFile = (e: React.MouseEvent) => {
    e.stopPropagation()
    onAddChild(node, 'file')
  }

  const handleAddFolder = (e: React.MouseEvent) => {
    e.stopPropagation()
    onAddChild(node, 'directory')
  }

  if (isEditing) {
    return (
      <InlineEditInput
        initialValue={node.name}
        depth={depth}
        type={node.type}
        blockType={node.blockType}
        onSave={(newName) => onRename(node, newName)}
        onCancel={onCancelRename}
      />
    )
  }

  const Icon =
    node.type === 'directory'
      ? Folder
      : node.blockType === 'task'
        ? CheckSquare
        : node.blockType === 'markdown'
          ? FileText
          : FileCode

  return (
    <div className="w-full">
      <div
        className={cn(
          'group flex w-full cursor-pointer items-center overflow-hidden rounded-md py-1 pr-1 transition-colors',
          isSelected
            ? 'bg-accent/10 font-medium text-accent'
            : 'text-foreground hover:bg-muted/50'
        )}
        style={{ paddingLeft: `${8 + depth * 16}px` }}
        onClick={handleSelect}
      >
        {/* Toggle Arrow */}
        <button
          onClick={handleToggle}
          className={cn(
            'flex h-5 w-5 flex-shrink-0 items-center justify-center rounded transition-colors hover:bg-muted/80',
            node.type !== 'directory' && 'pointer-events-none opacity-0'
          )}
        >
          {isExpanded ? (
            <ChevronDown className="h-3.5 w-3.5 text-muted-foreground" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5 text-muted-foreground" />
          )}
        </button>

        {/* Icon */}
        <Icon
          className={cn(
            'ml-0.5 h-4 w-4 flex-shrink-0',
            isSelected ? 'text-accent' : 'text-muted-foreground'
          )}
        />

        {/* Name */}
        <div className="ml-2 mr-2 min-w-0 flex-1">
          <span className="block select-none truncate text-left text-sm">
            {node.name}
          </span>
        </div>

        {/* Actions */}
        <div className="ml-auto flex flex-shrink-0 items-center gap-0.5 opacity-0 transition-opacity group-hover:opacity-100">
          {node.type === 'directory' && (
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <button
                  className="flex h-6 w-6 items-center justify-center rounded text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
                  title="Add..."
                  onClick={(e) => e.stopPropagation()}
                >
                  <Plus className="h-3.5 w-3.5" />
                </button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-40">
                <DropdownMenuItem onClick={handleAddFile}>
                  <FileText className="mr-2 h-3.5 w-3.5" />
                  Add Document
                </DropdownMenuItem>
                <DropdownMenuItem onClick={handleAddFolder}>
                  <Folder className="mr-2 h-3.5 w-3.5" />
                  Add Folder
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          )}

          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <button
                className="flex h-6 w-6 items-center justify-center rounded text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
                title="More options"
                onClick={(e) => e.stopPropagation()}
              >
                <MoreHorizontal className="h-3.5 w-3.5" />
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-40">
              <DropdownMenuItem
                onClick={(e) => {
                  e.stopPropagation()
                  onStartRename(node.path)
                }}
              >
                <Pencil className="mr-2 h-3.5 w-3.5" />
                Rename
              </DropdownMenuItem>
              <DropdownMenuItem
                onClick={(e) => {
                  e.stopPropagation()
                  onExport(node)
                }}
              >
                <Download className="mr-2 h-3.5 w-3.5" />
                Export
              </DropdownMenuItem>
              <DropdownMenuItem
                onClick={(e) => {
                  e.stopPropagation()
                  onDelete(node)
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

      {/* Children */}
      {node.type === 'directory' && isExpanded && (
        <div className="w-full">
          {node.children.map((child) => (
            <TreeNode
              key={child.path}
              node={child}
              depth={depth + 1}
              activeBlockId={activeBlockId}
              editingPath={editingPath}
              expandedPaths={expandedPaths}
              onSelect={onSelect}
              onToggle={onToggle}
              onAddChild={onAddChild}
              onRename={onRename}
              onDelete={onDelete}
              onExport={onExport}
              onStartRename={onStartRename}
              onCancelRename={onCancelRename}
            />
          ))}
        </div>
      )}
    </div>
  )
}

// --- Main Component ---
interface VfsTreeProps {
  nodes: VfsNode[]
  activeBlockId: string | null
  onSelect: (node: VfsNode) => void
  onAddChild: (node: VfsNode, type: 'file' | 'directory') => void
  onRename: (node: VfsNode, newName: string) => void
  onDelete: (node: VfsNode) => void
  onExport: (node: VfsNode) => void
}

export const VfsTree = ({
  nodes,
  activeBlockId,
  onSelect,
  onAddChild,
  onRename,
  onDelete,
  onExport,
}: VfsTreeProps) => {
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(new Set())
  const [editingPath, setEditingPath] = useState<string | null>(null)

  const handleToggle = (path: string) => {
    setExpandedPaths((prev) => {
      const next = new Set(prev)
      if (next.has(path)) {
        next.delete(path)
      } else {
        next.add(path)
      }
      return next
    })
  }

  const handleStartRename = (path: string) => {
    setEditingPath(path)
  }

  const handleCancelRename = () => {
    setEditingPath(null)
  }

  const handleRenameInternal = (node: VfsNode, newName: string) => {
    if (newName && newName !== node.name) {
      onRename(node, newName)
    }
    setEditingPath(null)
  }

  return (
    <div className="w-full space-y-0.5">
      {nodes.map((node) => (
        <TreeNode
          key={node.path}
          node={node}
          depth={0}
          activeBlockId={activeBlockId}
          editingPath={editingPath}
          expandedPaths={expandedPaths}
          onSelect={onSelect}
          onToggle={handleToggle}
          onAddChild={onAddChild}
          onRename={handleRenameInternal}
          onDelete={onDelete}
          onExport={onExport}
          onStartRename={handleStartRename}
          onCancelRename={handleCancelRename}
        />
      ))}
    </div>
  )
}
