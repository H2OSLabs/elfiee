import { useState, useMemo } from 'react'
import { Globe, User, Bot, FolderOpen } from 'lucide-react'
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
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { useAppStore } from '@/lib/app-store'
import { toast } from 'sonner'

interface GlobalCollaboratorDialogProps {
  fileId: string
  open: boolean
  onOpenChange: (open: boolean) => void
}

export const GlobalCollaboratorDialog = ({
  fileId,
  open,
  onOpenChange,
}: GlobalCollaboratorDialogProps) => {
  const {
    getEditors,
    createEditor,
    addGlobalCollaborator,
    isGlobalCollaborator,
    createAgent,
  } = useAppStore()

  const [activeTab, setActiveTab] = useState<'existing' | 'new'>('existing')
  const [selectedEditorId, setSelectedEditorId] = useState<string>('')
  const [newEditorName, setNewEditorName] = useState('')
  const [newEditorType, setNewEditorType] = useState<'Human' | 'Bot'>('Human')
  const [configDir, setConfigDir] = useState('')
  const [provider, setProvider] = useState('claude_code')
  const [isProcessing, setIsProcessing] = useState(false)

  const allEditors = getEditors(fileId)

  // Filter: only show editors that are NOT already global collaborators
  const availableEditors = useMemo(() => {
    return allEditors.filter((e) => !isGlobalCollaborator(fileId, e.editor_id))
  }, [allEditors, fileId, isGlobalCollaborator])

  const handleOpenChange = (nextOpen: boolean) => {
    if (!nextOpen) {
      setSelectedEditorId('')
      setNewEditorName('')
      setNewEditorType('Human')
      setConfigDir('')
      setProvider('claude_code')
      setActiveTab('existing')
    }
    onOpenChange(nextOpen)
  }

  const handleAddExisting = async () => {
    if (!selectedEditorId) return

    setIsProcessing(true)
    try {
      await addGlobalCollaborator(fileId, selectedEditorId)
      handleOpenChange(false)
    } catch (error) {
      console.error(error)
    } finally {
      setIsProcessing(false)
    }
  }

  const handleCreateNew = async () => {
    if (!newEditorName.trim()) return
    if (newEditorType === 'Bot' && !configDir.trim()) {
      toast.error('Config directory is required for Bot type')
      return
    }

    setIsProcessing(true)
    try {
      const newEditor = await createEditor(
        fileId,
        newEditorName.trim(),
        newEditorType
      )
      await addGlobalCollaborator(fileId, newEditor.editor_id)

      // For Bot editors, also create the agent block
      if (newEditorType === 'Bot') {
        await createAgent(
          fileId,
          configDir.trim(),
          newEditorName.trim(),
          newEditor.editor_id,
          provider
        )
      }

      handleOpenChange(false)
    } catch (error) {
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
            <Globe className="h-5 w-5" />
            Add Global Collaborator
          </DialogTitle>
          <DialogDescription>
            Grant wildcard permissions on all blocks. The collaborator can still
            be restricted per-block via revoke.
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
                <Label htmlFor="global-editor-select">Select User</Label>
                {availableEditors.length === 0 ? (
                  <div className="flex flex-col items-center justify-center rounded-md border border-dashed p-4 text-center">
                    <p className="text-sm text-muted-foreground">
                      No users available.
                    </p>
                    <p className="mt-1 text-xs text-muted-foreground/70">
                      All editors already have global access.
                    </p>
                  </div>
                ) : (
                  <Select
                    value={selectedEditorId}
                    onValueChange={setSelectedEditorId}
                    disabled={isProcessing}
                  >
                    <SelectTrigger id="global-editor-select">
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
                <Label htmlFor="global-name">Name</Label>
                <Input
                  id="global-name"
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
                    <RadioGroupItem value="Human" id="global-r-human" />
                    <Label
                      htmlFor="global-r-human"
                      className="flex cursor-pointer items-center gap-1.5 font-normal"
                    >
                      <User className="h-4 w-4 text-blue-500" />
                      Human
                    </Label>
                  </div>
                  <div className="flex items-center space-x-2 rounded-md border p-2 hover:bg-muted/50">
                    <RadioGroupItem value="Bot" id="global-r-bot" />
                    <Label
                      htmlFor="global-r-bot"
                      className="flex cursor-pointer items-center gap-1.5 font-normal"
                    >
                      <Bot className="h-4 w-4 text-purple-500" />
                      Bot
                    </Label>
                  </div>
                </RadioGroup>
              </div>

              {/* Bot-specific fields: config_dir + provider */}
              {newEditorType === 'Bot' && (
                <>
                  <div className="space-y-2">
                    <Label htmlFor="global-config-dir">Config Directory</Label>
                    <div className="flex gap-2">
                      <Input
                        id="global-config-dir"
                        placeholder="/path/to/project/.claude"
                        value={configDir}
                        onChange={(e) => setConfigDir(e.target.value)}
                        disabled={isProcessing}
                        className="flex-1"
                      />
                      <Button
                        variant="outline"
                        size="icon"
                        title="Browse..."
                        disabled={isProcessing}
                        onClick={async () => {
                          try {
                            const selected = await openDialog({
                              directory: true,
                              multiple: false,
                              title: 'Select AI tool config directory',
                            })
                            if (selected && typeof selected === 'string') {
                              setConfigDir(selected)
                            }
                          } catch {
                            // User cancelled
                          }
                        }}
                      >
                        <FolderOpen className="h-4 w-4" />
                      </Button>
                    </div>
                    <p className="text-[11px] text-muted-foreground">
                      Path to the AI tool&apos;s config directory (e.g.{' '}
                      <code className="rounded bg-muted px-1">.claude</code>,{' '}
                      <code className="rounded bg-muted px-1">.cursor</code>)
                    </p>
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="global-provider">Provider</Label>
                    <Select
                      value={provider}
                      onValueChange={setProvider}
                      disabled={isProcessing}
                    >
                      <SelectTrigger id="global-provider">
                        <SelectValue placeholder="Select provider" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="claude_code">Claude Code</SelectItem>
                        <SelectItem value="cursor">Cursor</SelectItem>
                        <SelectItem value="windsurf">Windsurf</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                </>
              )}
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
                : !newEditorName.trim() ||
                  (newEditorType === 'Bot' && !configDir.trim()))
            }
          >
            {isProcessing ? (
              <>
                <div className="mr-2 h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent" />
                {activeTab === 'existing' ? 'Adding...' : 'Creating...'}
              </>
            ) : activeTab === 'existing' ? (
              'Add as Global'
            ) : (
              'Create & Add as Global'
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
