import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from '@/components/ui/dialog'
import { Label } from '@/components/ui/label'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { FolderOpen, Cpu } from 'lucide-react'
import type { AgentContents, Block } from '@/bindings'

interface ConfigureBotDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  botName: string
  agentBlock?: Block
}

const PROVIDER_LABELS: Record<string, string> = {
  claude_code: 'Claude Code',
  cursor: 'Cursor',
  windsurf: 'Windsurf',
}

export function ConfigureBotDialog({
  open,
  onOpenChange,
  botName,
  agentBlock,
}: ConfigureBotDialogProps) {
  const agentContents = agentBlock?.contents as AgentContents | undefined

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[425px]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Cpu className="h-5 w-5" />
            Agent Configuration
          </DialogTitle>
          <DialogDescription>
            Configuration for <strong>{botName}</strong>
          </DialogDescription>
        </DialogHeader>

        {agentContents ? (
          <div className="grid gap-4 py-4">
            <div className="grid gap-1.5">
              <Label className="text-muted-foreground">Status</Label>
              <Badge
                variant={
                  agentContents.status === 'enabled' ? 'default' : 'secondary'
                }
                className={`w-fit ${
                  agentContents.status === 'enabled'
                    ? 'bg-green-100 text-green-700 hover:bg-green-100/80 dark:bg-green-900/30 dark:text-green-400'
                    : ''
                }`}
              >
                {agentContents.status}
              </Badge>
            </div>

            <div className="grid gap-1.5">
              <Label className="text-muted-foreground">Config Directory</Label>
              <div className="flex items-center gap-2 rounded-md border bg-muted/30 px-3 py-2">
                <FolderOpen className="h-4 w-4 shrink-0 text-muted-foreground" />
                <code className="truncate text-sm">
                  {agentContents.config_dir}
                </code>
              </div>
            </div>

            <div className="grid gap-1.5">
              <Label className="text-muted-foreground">Provider</Label>
              <p className="text-sm">
                {PROVIDER_LABELS[agentContents.provider ?? ''] ??
                  agentContents.provider ??
                  'Not set'}
              </p>
            </div>

            {agentContents.editor_id && (
              <div className="grid gap-1.5">
                <Label className="text-muted-foreground">Editor ID</Label>
                <code className="truncate text-xs text-muted-foreground">
                  {agentContents.editor_id}
                </code>
              </div>
            )}
          </div>
        ) : (
          <div className="flex flex-col items-center justify-center rounded-md border border-dashed p-6 text-center">
            <Cpu className="mb-2 h-8 w-8 text-muted-foreground/50" />
            <p className="text-sm text-muted-foreground">
              No agent block created yet.
            </p>
            <p className="mt-1 text-xs text-muted-foreground/70">
              Toggle the Agent switch to create one.
            </p>
          </div>
        )}

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Close
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
