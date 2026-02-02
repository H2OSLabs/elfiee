import { useState, useMemo } from 'react'
import { UserPlus, User, Bot } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Label } from '@/components/ui/label'
import { Input } from '@/components/ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group'
import { useAppStore } from '@/lib/app-store'
import type { Editor } from '@/bindings'
import { toast } from 'sonner'

interface AddCollaboratorDialogProps {
  fileId: string
  blockId: string
  blockType: string // Block type to determine default permission
  existingEditors: Editor[] // Editors who already have access to this block
  allEditors: Editor[] // All editors in the file system
  open: boolean
  onOpenChange: (open: boolean) => void
  onSuccess?: (editor: Editor) => void
}

// Get default read permission based on block type
const getDefaultReadPermission = (blockType: string): string => {
  if (blockType === 'code') return 'code.read'
  if (blockType === 'directory') return 'directory.read'
  if (blockType === 'task') return 'task.read'
  return 'markdown.read' // Default for markdown and other types
}

// Get default write permission based on block type
const getDefaultWritePermission = (blockType: string): string => {
  if (blockType === 'code') return 'code.write'
  if (blockType === 'directory') return 'directory.write'
  if (blockType === 'task') return 'task.write'
  return 'markdown.write' // Default for markdown and other types
}

export const AddCollaboratorDialog = ({
  fileId,
  blockId,
  blockType,
  existingEditors,
  allEditors,
  open,
  onOpenChange,
  onSuccess,
}: AddCollaboratorDialogProps) => {
  const { createEditor, grantCapability, getActiveEditor } = useAppStore()
  const [activeTab, setActiveTab] = useState<'existing' | 'new'>('existing')

  // Existing User State
  const [selectedEditorId, setSelectedEditorId] = useState<string>('')

  // New User State
  const [newEditorName, setNewEditorName] = useState('')
  const [newEditorType, setNewEditorType] = useState<'Human' | 'Bot'>('Human')

  const [isProcessing, setIsProcessing] = useState(false)

  // Filter: Available editors are those in the file system but NOT in the existing list for this block
  const availableEditors = useMemo(() => {
    const existingIds = new Set(existingEditors.map((e) => e.editor_id))
    return allEditors.filter((e) => !existingIds.has(e.editor_id))
  }, [allEditors, existingEditors])

  // Reset form when dialog opens/closes
  const handleOpenChange = (open: boolean) => {
    if (!open) {
      setSelectedEditorId('')
      setNewEditorName('')
      setNewEditorType('Human')
      setActiveTab('existing')
    }
    onOpenChange(open)
  }

  const handleAddExisting = async () => {
    if (!selectedEditorId) return
    const selectedEditor = availableEditors.find(
      (e) => e.editor_id === selectedEditorId
    )
    if (!selectedEditor) return

    setIsProcessing(true)
    try {
      // Grant permissions: core.read + type-specific read permission
      // core.read allows viewing metadata/events/grants
      // type-specific read allows viewing block content
      const defaultPermission = getDefaultReadPermission(blockType)
      const granterId = getActiveEditor(fileId)?.editor_id

      // Grant core.read first
      await grantCapability(
        fileId,
        selectedEditor.editor_id,
        'core.read',
        blockId,
        granterId
      )

      // Grant type-specific read permission
      await grantCapability(
        fileId,
        selectedEditor.editor_id,
        defaultPermission,
        blockId,
        granterId
      )

      // Grant type-specific write permission
      const defaultWritePermission = getDefaultWritePermission(blockType)
      await grantCapability(
        fileId,
        selectedEditor.editor_id,
        defaultWritePermission,
        blockId,
        granterId
      )

      toast.success(`Added ${selectedEditor.name} to collaborators`)
      onSuccess?.(selectedEditor)
      handleOpenChange(false)
    } catch (error) {
      // Backend validation errors (e.g. permission denied) will be caught here
      // toast error is already handled in app-store but we can log it
      console.error(error)
    } finally {
      setIsProcessing(false)
    }
  }

  const handleCreateNew = async () => {
    if (!newEditorName.trim()) return

    setIsProcessing(true)
    try {
      // 1. Create the editor
      const newEditor = await createEditor(
        fileId,
        newEditorName.trim(),
        newEditorType
      )

      // 2. Grant permissions: core.read + type-specific read permission
      const defaultPermission = getDefaultReadPermission(blockType)
      const granterId = getActiveEditor(fileId)?.editor_id

      // Grant core.read first
      await grantCapability(
        fileId,
        newEditor.editor_id,
        'core.read',
        blockId,
        granterId
      )

      // Grant type-specific read permission
      await grantCapability(
        fileId,
        newEditor.editor_id,
        defaultPermission,
        blockId,
        granterId
      )

      // Grant type-specific write permission
      const defaultWritePermission = getDefaultWritePermission(blockType)
      await grantCapability(
        fileId,
        newEditor.editor_id,
        defaultWritePermission,
        blockId,
        granterId
      )

      toast.success(`Created and added ${newEditor.name}`)
      onSuccess?.(newEditor)
      handleOpenChange(false)
    } catch (error) {
      // Backend validation errors will be displayed by app-store toast
      console.error(error)
    } finally {
      setIsProcessing(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="sm:max-w-[425px]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <UserPlus className="h-5 w-5" />
            Add Collaborator
          </DialogTitle>
          <DialogDescription>
            Grant access to an existing user or create a new collaborator
            identity.
          </DialogDescription>
        </DialogHeader>

        <Tabs
          value={activeTab}
          onValueChange={(v) => setActiveTab(v as 'existing' | 'new')}
          className="w-full"
        >
          <TabsList className="grid w-full grid-cols-2 bg-muted">
            <TabsTrigger
              value="existing"
              className="data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=inactive]:text-muted-foreground"
            >
              Select Existing
            </TabsTrigger>
            <TabsTrigger
              value="new"
              className="data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=inactive]:text-muted-foreground"
            >
              Create New
            </TabsTrigger>
          </TabsList>

          <div className="py-4">
            <TabsContent value="existing" className="mt-0 space-y-4">
              <div className="space-y-2">
                <Label htmlFor="editor-select">Select User</Label>
                {availableEditors.length === 0 ? (
                  <div className="flex flex-col items-center justify-center rounded-md border border-dashed p-4 text-center">
                    <p className="text-sm text-muted-foreground">
                      No other users available.
                    </p>
                    <p className="mt-1 text-xs text-muted-foreground/70">
                      Everyone in this file already has access.
                    </p>
                  </div>
                ) : (
                  <Select
                    value={selectedEditorId}
                    onValueChange={setSelectedEditorId}
                    disabled={isProcessing}
                  >
                    <SelectTrigger id="editor-select">
                      <SelectValue placeholder="Choose a user..." />
                    </SelectTrigger>
                    <SelectContent>
                      {availableEditors.map((editor) => (
                        <SelectItem
                          key={editor.editor_id}
                          value={editor.editor_id}
                        >
                          <div className="flex items-center gap-2">
                            {editor.editor_type === 'Bot' ? (
                              <Bot className="h-4 w-4 text-purple-500" />
                            ) : (
                              <User className="h-4 w-4 text-blue-500" />
                            )}
                            <span className="font-medium">{editor.name}</span>
                          </div>
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                )}
              </div>
            </TabsContent>

            <TabsContent value="new" className="mt-0 space-y-4">
              <div className="space-y-2">
                <Label htmlFor="name">Name</Label>
                <Input
                  id="name"
                  placeholder="e.g. Alice, ReviewerBot"
                  value={newEditorName}
                  onChange={(e) => setNewEditorName(e.target.value)}
                  disabled={isProcessing}
                  onKeyDown={(e) => {
                    if (
                      e.key === 'Enter' &&
                      newEditorName.trim() &&
                      !isProcessing
                    ) {
                      e.preventDefault()
                      handleCreateNew()
                    }
                  }}
                />
              </div>
              <div className="space-y-2">
                <Label>Type</Label>
                <RadioGroup
                  value={newEditorType}
                  onValueChange={(v) => setNewEditorType(v as 'Human' | 'Bot')}
                  className="flex gap-4"
                  disabled={isProcessing}
                >
                  <div className="flex items-center space-x-2 rounded-md border p-2 hover:bg-muted/50">
                    <RadioGroupItem value="Human" id="r-human" />
                    <Label
                      htmlFor="r-human"
                      className="flex cursor-pointer items-center gap-1.5 font-normal"
                    >
                      <User className="h-4 w-4 text-blue-500" />
                      Human
                    </Label>
                  </div>
                  <div className="flex items-center space-x-2 rounded-md border p-2 hover:bg-muted/50">
                    <RadioGroupItem value="Bot" id="r-bot" />
                    <Label
                      htmlFor="r-bot"
                      className="flex cursor-pointer items-center gap-1.5 font-normal"
                    >
                      <Bot className="h-4 w-4 text-purple-500" />
                      Bot
                    </Label>
                  </div>
                </RadioGroup>
              </div>
            </TabsContent>
          </div>
        </Tabs>

        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            onClick={() => handleOpenChange(false)}
            disabled={isProcessing}
          >
            Cancel
          </Button>
          <Button
            type="button"
            onClick={
              activeTab === 'existing' ? handleAddExisting : handleCreateNew
            }
            disabled={
              isProcessing ||
              (activeTab === 'existing'
                ? !selectedEditorId
                : !newEditorName.trim())
            }
          >
            {isProcessing ? (
              <>
                <div className="mr-2 h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent" />
                {activeTab === 'existing' ? 'Adding...' : 'Creating...'}
              </>
            ) : activeTab === 'existing' ? (
              'Add'
            ) : (
              'Create & Add'
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
