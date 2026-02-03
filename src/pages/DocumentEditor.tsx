import { Sidebar } from '@/components/dashboard/Sidebar'
import { FilePanel } from '@/components/editor/FilePanel'
import { EditorCanvas } from '@/components/editor/EditorCanvas'
import ContextPanel from '@/components/editor/ContextPanel'
import { Menu } from 'lucide-react'
import { useState, useEffect } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { Sheet, SheetContent, SheetTrigger } from '@/components/ui/sheet'
import { Button } from '@/components/ui/button'
import { useAppStore } from '@/lib/app-store'
import { toast } from 'sonner'
import { events } from '@/bindings'

const DocumentEditor = () => {
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false)
  const { projectId } = useParams<{ projectId: string }>()
  const navigate = useNavigate()
  const [isLoading, setIsLoading] = useState(true)
  const [loadedFileId, setLoadedFileId] = useState<string | null>(null)

  // Load file when projectId changes
  useEffect(() => {
    let outputMounted = true

    const loadFile = async () => {
      // Skip if no project ID
      if (!projectId) {
        setIsLoading(false)
        return
      }

      try {
        setIsLoading(true)

        // Get store state
        const store = useAppStore.getState()
        const files = store.files

        // Check if file is already in store (from Projects page)
        const fileState = files.get(projectId)
        if (fileState) {
          // File already loaded in store, just set it as current
          store.setCurrentFile(projectId)
        } else {
          // File not in store, need to fetch metadata and initialize
          // This happens when user directly navigates to /editor/:fileId
          const metadata = await store.getFileInfo(projectId)

          // Initialize file state in store
          const newFiles = new Map(files)
          newFiles.set(projectId, {
            fileId: projectId,
            metadata,
            editors: [],
            activeEditorId: null,
            blocks: [],
            selectedBlockId: null,
            events: [],
            grants: [],
          })

          // Update store with new file state
          useAppStore.setState({ files: newFiles, currentFileId: projectId })
        }

        // Load blocks, editors, grants, and events for this file
        await store.loadBlocks(projectId)
        await store.loadEditors(projectId)
        await store.loadGrants(projectId)
        await store.loadEvents(projectId)

        // Mark as loaded
        if (outputMounted) {
          setLoadedFileId(projectId)
        }
      } catch (error) {
        console.error('Failed to load file:', error)
        toast.error(`Failed to load file: ${error}`)
        // Navigate back to projects page on error
        if (outputMounted) {
          navigate('/')
        }
      } finally {
        if (outputMounted) {
          setIsLoading(false)
        }
      }
    }

    loadFile()

    // Cleanup: Save and Close file on unmount
    return () => {
      outputMounted = false
      if (projectId) {
        // We use a self-executing async function for cleanup
        ;(async () => {
          try {
            console.log(
              `[DocumentEditor] Unmounting, saving file: ${projectId}`
            )
            // Save pending changes
            const store = useAppStore.getState()
            await store.saveFile(projectId)

            // NOTE: Temporarily disabled closeFile on unmount because in React Strict Mode (dev),
            // components mount/unmount/mount rapidly. This causes the file to be closed
            // immediately after opening, breaking the session for the second mount.
            // For now, we leave the file open. The backend cleans up on app exit.
            // await TauriClient.file.closeFile(projectId)

            // Update store state if needed (optional, but good practice to clear current)
            if (store.currentFileId === projectId) {
              store.setCurrentFile(null)
            }
          } catch (error) {
            console.error(
              '[DocumentEditor] Failed to save/close file on exit:',
              error
            )
          }
        })()
      }
    }
  }, [projectId])

  // Auto-refresh when backend state changes (e.g., MCP/Claude Code modifications)
  useEffect(() => {
    const unlisten = events.stateChangedEvent.listen((event) => {
      const { file_id } = event.payload
      if (file_id === projectId) {
        const store = useAppStore.getState()
        store.loadBlocks(file_id)
        store.loadGrants(file_id)
        store.loadEvents(file_id)
      }
    })

    return () => {
      unlisten.then((f) => f())
    }
  }, [projectId])

  // Show loading state
  if (isLoading) {
    return (
      <div className="flex h-screen w-full items-center justify-center bg-background">
        <div className="text-center">
          <div className="mb-4 text-lg text-muted-foreground">
            Loading project...
          </div>
        </div>
      </div>
    )
  }

  // Show error if no projectId
  if (!projectId && !loadedFileId) {
    return (
      <div className="flex h-screen w-full items-center justify-center bg-background">
        <div className="text-center">
          <div className="mb-4 text-lg text-muted-foreground">
            No project selected
          </div>
          <Button onClick={() => navigate('/')}>Go to Projects</Button>
        </div>
      </div>
    )
  }

  return (
    <div className="flex h-screen w-full overflow-hidden bg-white">
      {/* Mobile Header - Only visible on mobile */}
      <div className="fixed left-0 right-0 top-0 z-50 flex h-14 items-center border-b border-border bg-background px-4 lg:hidden">
        <Sheet open={mobileMenuOpen} onOpenChange={setMobileMenuOpen}>
          <SheetTrigger asChild>
            <Button variant="ghost" size="icon" className="text-accent">
              <Menu className="h-6 w-6" />
            </Button>
          </SheetTrigger>
          <SheetContent
            side="left"
            className="flex h-full w-[400px] flex-col overflow-hidden p-0 sm:w-[450px]"
          >
            <div className="flex min-h-0 flex-1 overflow-hidden">
              <Sidebar />
              <div className="min-h-0 min-w-0 flex-1 overflow-hidden">
                <FilePanel />
              </div>
            </div>
          </SheetContent>
        </Sheet>
        <span className="ml-3 text-sm font-medium">Document Editor</span>
      </div>

      {/* Desktop Layout - Fluid 3-Column Flexbox */}

      {/* Column 1: Global Nav (Fixed ~80px) - Always visible on desktop */}
      <div className="hidden h-full w-20 shrink-0 lg:block">
        <Sidebar />
      </div>

      {/* Column 2: File Panel - Fixed width, NEVER shrinks */}
      <div className="hidden h-full min-w-[240px] max-w-[300px] shrink-0 overflow-hidden border-r border-gray-200 bg-[#F9FAFB] lg:flex">
        <FilePanel />
      </div>

      {/* Column 3: Main Editor Canvas - FLUID, absorbs ALL shrinkage */}
      <div className="flex h-full min-w-0 flex-1 flex-col overflow-hidden pt-14 lg:pt-0">
        <EditorCanvas />
      </div>

      {/* Column 4: Context Panel - Fixed width, NEVER shrinks */}
      <div className="hidden h-full w-80 shrink-0 overflow-hidden border-l border-gray-200 bg-white lg:flex">
        <ContextPanel />
      </div>
    </div>
  )
}

export default DocumentEditor
