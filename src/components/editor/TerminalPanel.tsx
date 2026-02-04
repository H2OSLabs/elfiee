import { useEffect, useRef, useState } from 'react'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { useAppStore } from '@/lib/app-store'
import { events } from '@/bindings'
import { Button } from '@/components/ui/button'
import {
  X,
  Minimize2,
  Maximize2,
  Terminal as TerminalIcon,
  Trash2,
} from 'lucide-react'
import { toast } from 'sonner'
import { ConfirmDeleteDialog } from './ConfirmDeleteDialog'
import '@xterm/xterm/css/xterm.css'
import './terminal-panel.css'

interface TerminalPanelProps {
  fileId: string
  onClose: () => void
}

export function TerminalPanel({ fileId, onClose }: TerminalPanelProps) {
  const terminalRef = useRef<HTMLDivElement>(null)
  const xtermRef = useRef<Terminal | null>(null)
  const fitAddonRef = useRef<FitAddon | null>(null)
  const [isMaximized, setIsMaximized] = useState(false)
  const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false)
  const [blockId, setBlockId] = useState<string | null>(null)
  const [isInitializing, setIsInitializing] = useState(true)
  const [editorId, setEditorId] = useState<string | null>(null)
  const saveIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const isPtyInitialized = useRef<boolean>(false)
  const isBlockCreated = useRef<boolean>(false)

  // Zustand Actions
  const createTerminalBlock = useAppStore((state) => state.createTerminalBlock)
  const initTerminal = useAppStore((state) => state.initTerminal)
  const writeToPty = useAppStore((state) => state.writeToPty)
  const resizePty = useAppStore((state) => state.resizePty)
  const saveTerminal = useAppStore((state) => state.saveTerminal)
  const closeTerminalSession = useAppStore(
    (state) => state.closeTerminalSession
  )
  const getActiveEditor = useAppStore((state) => state.getActiveEditor)
  const selectedBlockId = useAppStore((state) => state.selectedBlockId)
  const getBlock = useAppStore((state) => state.getBlock)
  const fetchBlock = useAppStore((state) => state.fetchBlock)
  const deleteBlock = useAppStore((state) => state.deleteBlock)

  // Handle switching active terminal when selectedBlockId changes
  useEffect(() => {
    if (!selectedBlockId) return

    const block = getBlock(fileId, selectedBlockId)
    // Only switch if it's a DIFFERENT terminal block
    if (
      block &&
      block.block_type === 'terminal' &&
      block.block_id !== blockId
    ) {
      console.log(`[Terminal] Switching from ${blockId} to ${block.block_id}`)

      const switchTerminalSession = async () => {
        // 1. Save current session content BEFORE switching context
        if (xtermRef.current && blockId) {
          try {
            const buffer = xtermRef.current.buffer.active
            const lines: string[] = []
            for (let i = 0; i < buffer.length; i++) {
              const line = buffer.getLine(i)
              if (line) {
                lines.push(line.translateToString(true))
              }
            }
            const content = lines.join('\n')
            const trimmedContent = content.replace(/\s+$/, '')

            console.log(
              `[Terminal] Saving ${blockId} before switch, content length: ${trimmedContent.length}`
            )
            await saveTerminal(
              fileId,
              blockId,
              trimmedContent,
              editorId || undefined
            )
          } catch (err) {
            console.error('Failed to save terminal before switch:', err)
          }
        }

        // 2. Prepare for new context
        // We DON'T set isInitializing(true) here to avoid the major UI flash/flicker.
        // Changing blockId will trigger the main terminal init effect which handles cleanup/re-init.
        isPtyInitialized.current = false

        // 3. Update to new block context
        setEditorId(block.owner)
        setBlockId(block.block_id)
        isBlockCreated.current = true
      }

      switchTerminalSession()
    }
  }, [selectedBlockId, fileId, blockId, getBlock, saveTerminal, editorId])

  // Create or Load Terminal Block on mount
  useEffect(() => {
    // Prevent duplicate block creation
    if (isBlockCreated.current) {
      return
    }

    let isMounted = true

    const initTerminalBlock = async () => {
      try {
        // 1. Check if we should open an existing terminal (from selection)
        if (selectedBlockId) {
          const block = getBlock(fileId, selectedBlockId)
          if (block && block.block_type === 'terminal') {
            if (isMounted) {
              setEditorId(block.owner)
              setBlockId(block.block_id)
              setIsInitializing(false)
              isBlockCreated.current = true
              return
            }
          }
        }

        // 2. Otherwise create a new terminal session
        // Get current active editor - use for both Block creation and PTY init
        const activeEditor = getActiveEditor(fileId)
        if (!activeEditor) {
          throw new Error('No active editor found')
        }

        // Only update state if component is still mounted
        if (!isMounted) return

        setEditorId(activeEditor.editor_id)

        const newBlockId = await createTerminalBlock(
          fileId,
          'Terminal Session',
          activeEditor.editor_id
        )

        // Only update state if component is still mounted
        if (!isMounted) return

        setBlockId(newBlockId)
        setIsInitializing(false)
        isBlockCreated.current = true
      } catch (error) {
        if (!isMounted) return
        console.error('Failed to create terminal block:', error)
        toast.error('Failed to create terminal session')
        onClose()
      }
    }

    initTerminalBlock()

    return () => {
      isMounted = false
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [fileId]) // Run once on mount (dependency fileId)

  // Initialize terminal after block is created
  useEffect(() => {
    if (!terminalRef.current || !blockId || !editorId || isInitializing) return

    // Capture current values at effect execution time to avoid stale closure issues
    // This ensures cleanup saves to the correct block, not the new one after state update
    const currentBlockId = blockId
    const currentEditorId = editorId

    let isMounted = true

    // Create xterm instance with improved theme
    const term = new Terminal({
      fontFamily:
        '"Cascadia Code", "Fira Code", "JetBrains Mono", Consolas, monospace',
      fontSize: 13,
      fontWeight: '400',
      lineHeight: 1.4,
      theme: {
        background: '#0a0a0a',
        foreground: '#e4e4e7',
        cursor: '#22d3ee',
        cursorAccent: '#0a0a0a',
        selectionBackground: '#334155', // Fixed: use selectionBackground instead of selection
        black: '#27272a',
        brightBlack: '#52525b',
        red: '#f87171',
        brightRed: '#fca5a5',
        green: '#4ade80',
        brightGreen: '#86efac',
        yellow: '#fbbf24',
        brightYellow: '#fde047',
        blue: '#60a5fa',
        brightBlue: '#93c5fd',
        magenta: '#c084fc',
        brightMagenta: '#d8b4fe',
        cyan: '#22d3ee',
        brightCyan: '#67e8f9',
        white: '#d4d4d8',
        brightWhite: '#fafafa',
      },
      scrollback: 10000,
      cursorBlink: true,
      cursorStyle: 'bar',
      allowTransparency: true,
      scrollOnUserInput: true, // Auto-scroll when user types
    })

    const fitAddon = new FitAddon()
    term.loadAddon(fitAddon)
    term.open(terminalRef.current)

    // Use requestAnimationFrame to ensure DOM is fully rendered before fitting
    requestAnimationFrame(() => {
      if (!term.element) return // Guard against early disposal
      fitAddon.fit()

      // Small delay to ensure xterm.js has calculated correct dimensions
      setTimeout(async () => {
        if (!isMounted || !term.element) return
        fitAddon.fit()
        xtermRef.current = term
        fitAddonRef.current = fitAddon

        // Initialize session (restore history + spawn PTY)
        try {
          // 1. Restore saved content (HISTORY FIRST)
          await restoreTerminalContent(term)

          // 2. Short pause to ensure history is rendered and buffered in xterm
          await new Promise((resolve) => setTimeout(resolve, 300))

          // 3. Initialize PTY session
          if (isMounted) {
            await initTerminalSession(term)
            // Mark PTY as initialized
            isPtyInitialized.current = true

            // After PTY init, send resize command
            await resizePty(
              fileId,
              currentBlockId,
              term.cols,
              term.rows,
              currentEditorId || undefined
            )
          }
        } catch (error) {
          if (!isMounted) return
          console.error(
            'Failed to initialize or restore Terminal session:',
            error
          )
          // Don't show toast for all errors to avoid noise
        }
      }, 50)
    })

    // Listen for PTY output (typed event from events.rs)
    const unlistenPromise = events.ptyOutputEvent.listen((event) => {
      if (event.payload.block_id === currentBlockId) {
        // Decode base64 to UTF-8 properly (atob only handles Latin-1, corrupting UTF-8)
        const binaryString = atob(event.payload.data)
        const bytes = new Uint8Array(binaryString.length)
        for (let i = 0; i < binaryString.length; i++) {
          bytes[i] = binaryString.charCodeAt(i)
        }
        const decoded = new TextDecoder('utf-8').decode(bytes)
        term.write(decoded)
      }
    })

    // Listen for user input
    const disposable = term.onData((data) => {
      writeToPty(
        fileId,
        currentBlockId,
        data,
        currentEditorId || undefined
      ).catch((error) => {
        console.error('Failed to write to PTY:', error)
      })
    })

    // Auto-save terminal content function
    // Uses captured currentBlockId/currentEditorId to ensure correct block is saved during cleanup
    const saveTerminalContent = async () => {
      try {
        const buffer = term.buffer.active
        const lines: string[] = []
        for (let i = 0; i < buffer.length; i++) {
          const line = buffer.getLine(i)
          if (line) {
            lines.push(line.translateToString(true))
          }
        }
        const content = lines.join('\n')
        // Trim trailing empty lines to avoid excessive scrolling on restore
        const trimmedContent = content.replace(/\s+$/, '')
        await saveTerminal(
          fileId,
          currentBlockId,
          trimmedContent,
          currentEditorId || undefined
        )
      } catch (error) {
        console.error('Failed to save terminal content:', error)
      }
    }

    // Listen for Enter key to trigger auto-save (without Shift for multi-line input in PowerShell)
    const keyDisposable = term.onKey(({ domEvent }) => {
      if (domEvent.key === 'Enter' && !domEvent.shiftKey) {
        // Delay save to allow command output to arrive
        setTimeout(() => saveTerminalContent(), 500)
      }
    })

    // Listen for xterm.js resize events (more reliable than window resize)
    const resizeDisposable = term.onResize(({ cols, rows }) => {
      // Only resize PTY if it has been initialized
      if (!isPtyInitialized.current) {
        return
      }
      resizePty(
        fileId,
        currentBlockId,
        cols,
        rows,
        currentEditorId || undefined
      ).catch((error) => {
        console.error('Failed to resize PTY:', error)
      })
    })

    // Window resize handler
    const handleResize = () => {
      fitAddon.fit()
    }
    window.addEventListener('resize', handleResize)

    // Save every 10 seconds
    saveIntervalRef.current = setInterval(saveTerminalContent, 10000)

    // Cleanup
    return () => {
      isMounted = false

      // Save one last time before cleanup
      saveTerminalContent()

      // Clear auto-save interval
      if (saveIntervalRef.current) {
        clearInterval(saveIntervalRef.current)
        saveIntervalRef.current = null
      }

      // Reset PTY initialization flag
      isPtyInitialized.current = false

      disposable.dispose()
      keyDisposable.dispose()
      resizeDisposable.dispose()
      unlistenPromise.then((fn) => fn())
      window.removeEventListener('resize', handleResize)
      term.dispose()
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [blockId, editorId, fileId, isInitializing])

  const initTerminalSession = async (term: Terminal) => {
    if (!blockId || !editorId) return

    try {
      // NOTE: restoreTerminalContent is now called EXPLICITLY before this in useEffect
      // This function now ONLY handles backend PTY initialization

      // Get block's _block_dir for working directory
      // This makes terminal start in the block's tmp directory and `cd ~` redirects there
      const block = await fetchBlock(fileId, blockId)
      const cwd = (block?.contents as any)?._block_dir as string | undefined

      // Initialize PTY session with the same editor that created the block
      await initTerminal(
        fileId,
        blockId,
        term.cols,
        term.rows,
        cwd, // Pass the block's _block_dir as cwd
        editorId // Use the same editor ID that created the block
      )
    } catch (error) {
      console.error('Failed to init terminal:', error)
      toast.error('Failed to initialize terminal')
    }
  }

  const restoreTerminalContent = async (term: Terminal) => {
    if (!blockId) return

    try {
      console.log(`[Terminal] Restoring block ${blockId}...`)

      // 1. Fetch fresh block data from backend to ensure we have contents + permission
      const block = await fetchBlock(fileId, blockId)

      if (block && block.contents) {
        const contents = block.contents as any
        const savedContent = contents.saved_content

        console.log(`[Terminal] Fetched contents:`, contents)

        if (
          savedContent &&
          typeof savedContent === 'string' &&
          savedContent.trim()
        ) {
          console.log(
            `[Terminal] Writing ${savedContent.length} chars of history to xterm`
          )

          // Clear current terminal buffer just in case (though it should be fresh)
          term.clear()

          // Write saved content to terminal, replacing newlines with \r\n for xterm
          // We use a small delay between writes if history is huge? No, xterm handles it.
          term.write(savedContent.replace(/\r?\n/g, '\r\n'))

          // If the last line doesn't end with a newline, add one to separate history from prompt
          if (!savedContent.endsWith('\n')) {
            term.write('\r\n')
          }

          // Ensure we are scrolled to the bottom
          setTimeout(() => term.scrollToBottom(), 10)
        } else {
          console.log(`[Terminal] No history found or content is empty`)
        }
      } else {
        console.log(`[Terminal] Block not found or has no contents in store`)
      }
    } catch (error) {
      console.error('Failed to restore terminal content:', error)
    }
  }

  const handleToggleMaximize = () => {
    setIsMaximized(!isMaximized)
    setTimeout(() => fitAddonRef.current?.fit(), 300)
  }

  const handleClose = async () => {
    if (blockId) {
      try {
        // Save terminal content one last time
        if (xtermRef.current) {
          const buffer = xtermRef.current.buffer.active
          const lines: string[] = []
          for (let i = 0; i < buffer.length; i++) {
            const line = buffer.getLine(i)
            if (line) {
              lines.push(line.translateToString(true))
            }
          }
          const content = lines.join('\n')
          // Trim trailing empty lines to avoid excessive scrolling on restore
          const trimmedContent = content.replace(/\s+$/, '')
          await saveTerminal(
            fileId,
            blockId,
            trimmedContent,
            editorId || undefined
          )
        }

        // Close PTY session
        await closeTerminalSession(fileId, blockId, editorId || undefined)
      } catch (error) {
        console.error('Failed to close terminal:', error)
      }
    }
    onClose()
  }

  const confirmDelete = async () => {
    if (!blockId) return

    try {
      // 1. Physical Cleanup: Close PTY
      await closeTerminalSession(fileId, blockId, editorId || undefined).catch(
        () => {}
      )

      // 2. Data Deletion: Remove Block (also clears selection in app-store)
      await deleteBlock(fileId, blockId)

      // 3. UI Cleanup: Close Panel
      onClose()
    } catch (error) {
      console.error('Failed to delete terminal block:', error)
      toast.error('Failed to delete terminal')
    } finally {
      setIsDeleteDialogOpen(false)
    }
  }

  return (
    <div
      className={`flex flex-col border-t border-zinc-800/50 bg-[#0a0a0a] transition-all duration-300 ${
        isMaximized ? 'h-[70vh]' : 'h-[300px]'
      }`}
    >
      {/* Header */}
      <div className="flex items-center justify-between border-b border-zinc-800/50 bg-zinc-950/80 px-3 py-1.5 backdrop-blur-sm">
        <div className="flex items-center gap-2">
          <TerminalIcon className="h-3.5 w-3.5 text-cyan-400" />
          <span className="text-xs font-medium text-zinc-400">Terminal</span>
          <span className="ml-2 rounded bg-zinc-800 px-1.5 py-0.5 font-mono text-[10px] text-zinc-500">
            {blockId ? blockId.slice(0, 8) : 'No Session'}
          </span>
        </div>
        <div className="flex gap-0.5">
          <Button
            size="sm"
            variant="ghost"
            onClick={() => setIsDeleteDialogOpen(true)}
            className="h-6 w-6 p-0 text-zinc-500 hover:bg-destructive/10 hover:text-destructive"
            title="Delete Terminal Session"
          >
            <Trash2 className="h-3 w-3" />
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={handleToggleMaximize}
            className="h-6 w-6 p-0 text-zinc-500 hover:bg-zinc-800/50 hover:text-zinc-300"
          >
            {isMaximized ? (
              <Minimize2 className="h-3 w-3" />
            ) : (
              <Maximize2 className="h-3 w-3" />
            )}
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={handleClose}
            className="h-6 w-6 p-0 text-zinc-500 hover:bg-red-500/10 hover:text-red-400"
          >
            <X className="h-3 w-3" />
          </Button>
        </div>
      </div>

      {/* Terminal Container */}
      {isInitializing ? (
        <div className="flex flex-1 items-center justify-center text-zinc-500">
          <div className="flex items-center gap-2">
            <div className="h-4 w-4 animate-spin rounded-full border-2 border-cyan-400 border-t-transparent"></div>
            <span className="text-sm">Initializing terminal session...</span>
          </div>
        </div>
      ) : (
        <div
          key={blockId}
          ref={terminalRef}
          className="terminal-container flex-1 overflow-hidden p-2"
        />
      )}

      <ConfirmDeleteDialog
        isOpen={isDeleteDialogOpen}
        onClose={() => setIsDeleteDialogOpen(false)}
        onConfirm={confirmDelete}
        title="Delete Terminal Session?"
        description="Are you sure you want to permanently delete this terminal? This action cannot be undone."
        details={
          <>
            <span className="text-sm font-semibold text-foreground">
              {blockId ? getBlock(fileId, blockId)?.name : 'Terminal Session'}
            </span>
            <span className="truncate font-mono text-[10px] text-muted-foreground">
              ID: {blockId}
            </span>
          </>
        }
      />
    </div>
  )
}
