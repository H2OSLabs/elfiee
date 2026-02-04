import { FolderKanban, FileEdit, User, X, UserPlus, Globe } from 'lucide-react'
import { NavLink } from '@/components/NavLink'
import { useAppStore } from '@/lib/app-store'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Avatar, AvatarFallback } from '@/components/ui/avatar'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { useEffect, useState } from 'react'
import { GlobalCollaboratorDialog } from '@/components/permission/GlobalCollaboratorDialog'

const navItems = [
  { icon: FolderKanban, path: '/', label: 'Projects' },
  { icon: FileEdit, path: '/editor', label: 'Editor' },
]

export const Sidebar = () => {
  const {
    currentFileId,
    getEditors,
    getActiveEditor,
    setActiveEditor,
    deleteEditor,
    getSystemEditorId,
    isGlobalCollaborator,
  } = useAppStore()

  const editors = currentFileId ? getEditors(currentFileId) : []
  const activeEditor = currentFileId ? getActiveEditor(currentFileId) : null

  // Get system editor ID from config (the local user/owner)
  const [systemEditorId, setSystemEditorId] = useState<string | null>(null)
  const [showGlobalDialog, setShowGlobalDialog] = useState(false)

  useEffect(() => {
    const loadSystemEditorId = async () => {
      try {
        const id = await getSystemEditorId()
        setSystemEditorId(id)
      } catch (error) {
        console.error('Failed to get system editor ID:', error)
      }
    }
    loadSystemEditorId()
  }, [getSystemEditorId])

  // Check if current user is the system owner
  const isSystemOwner = activeEditor?.editor_id === systemEditorId

  const handleDeleteUser = async (editorId: string) => {
    if (!currentFileId) return

    // Confirm deletion
    if (
      !confirm(
        'Are you sure you want to delete this user? This will remove all their access and history association.'
      )
    ) {
      return
    }

    try {
      await deleteEditor(currentFileId, editorId)
      // Success toast is handled by store
    } catch (error) {
      // Error toast is handled by store
      console.error(error)
    }
  }

  return (
    <aside className="z-50 flex h-full min-h-0 w-20 flex-shrink-0 flex-col items-center bg-primary py-6">
      {/* Logo */}
      <div className="mb-12">
        <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-accent">
          <span className="text-xl font-bold text-accent-foreground">A</span>
        </div>
      </div>

      {/* Navigation */}
      <nav className="flex flex-1 flex-col gap-6">
        {navItems.map((item) => (
          <NavLink
            key={item.path}
            to={item.path}
            className="group relative"
            activeClassName="after:absolute after:left-0 after:top-0 after:w-1 after:h-full after:bg-accent"
          >
            <div className="flex h-12 w-12 items-center justify-center text-primary-foreground/60 transition-colors group-hover:text-primary-foreground">
              <item.icon className="h-5 w-5" />
            </div>
          </NavLink>
        ))}
      </nav>

      {/* User Switcher */}
      <div className="mt-auto">
        {currentFileId && activeEditor ? (
          <DropdownMenu>
            <DropdownMenuTrigger className="outline-none">
              <div
                className="group flex h-12 w-12 items-center justify-center rounded-full transition-colors hover:bg-white/10"
                title={`Current user: ${activeEditor.name}`}
              >
                <Avatar className="h-9 w-9 border-2 border-transparent transition-all group-hover:border-accent">
                  <AvatarFallback className="bg-accent text-sm font-bold text-accent-foreground">
                    {activeEditor.name.substring(0, 2).toUpperCase()}
                  </AvatarFallback>
                </Avatar>
              </div>
            </DropdownMenuTrigger>
            <DropdownMenuContent side="right" align="end" className="ml-2 w-64">
              <DropdownMenuLabel>Switch User</DropdownMenuLabel>
              <DropdownMenuSeparator />
              {editors.map((editor) => {
                const isActive = editor.editor_id === activeEditor.editor_id
                // Check if this editor is the system owner (by ID, not name)
                const isSystemUser = editor.editor_id === systemEditorId
                // Can delete if: current user is system owner, not deleting self, not deleting the system owner
                const canDelete = isSystemOwner && !isActive && !isSystemUser
                const isGlobal = currentFileId
                  ? isGlobalCollaborator(currentFileId, editor.editor_id)
                  : false

                return (
                  <DropdownMenuItem
                    key={editor.editor_id}
                    className="flex items-center justify-between gap-2 p-2 focus:bg-accent/5"
                    // Prevent default click behavior on the item to handle children separately
                    onSelect={(e) => e.preventDefault()}
                  >
                    <div
                      className="flex flex-1 cursor-pointer items-center gap-2 overflow-hidden"
                      onClick={() =>
                        setActiveEditor(currentFileId, editor.editor_id)
                      }
                    >
                      <User
                        className={cn(
                          'h-4 w-4 shrink-0',
                          isActive ? 'text-primary' : 'text-muted-foreground'
                        )}
                      />
                      <span
                        className={cn(
                          'truncate text-sm',
                          isActive && 'font-semibold text-primary'
                        )}
                      >
                        {editor.name}
                      </span>
                      {isGlobal && (
                        <Badge
                          variant="secondary"
                          className="h-4 shrink-0 px-1.5 text-[10px]"
                        >
                          <Globe className="mr-0.5 h-2.5 w-2.5" />
                          Global
                        </Badge>
                      )}
                    </div>

                    <div className="flex items-center gap-1">
                      {isActive && (
                        <span className="h-2 w-2 rounded-full bg-green-500" />
                      )}

                      {canDelete && (
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-6 w-6 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
                          onClick={(e) => {
                            e.stopPropagation()
                            handleDeleteUser(editor.editor_id)
                          }}
                          title={
                            isSystemOwner
                              ? 'Delete User'
                              : 'Only the system owner can delete users'
                          }
                        >
                          <X className="h-3 w-3" />
                        </Button>
                      )}
                    </div>
                  </DropdownMenuItem>
                )
              })}
              <DropdownMenuSeparator />
              <DropdownMenuItem
                className="flex items-center gap-2 text-sm"
                onSelect={() => setShowGlobalDialog(true)}
              >
                <UserPlus className="h-4 w-4" />
                Add Global Collaborator
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        ) : (
          <div className="h-12 w-12" /> // Placeholder to maintain spacing
        )}
      </div>
      {currentFileId && (
        <GlobalCollaboratorDialog
          open={showGlobalDialog}
          onOpenChange={setShowGlobalDialog}
          fileId={currentFileId}
        />
      )}
    </aside>
  )
}
