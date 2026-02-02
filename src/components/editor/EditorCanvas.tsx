import { useMemo, useState, useEffect, useRef, useCallback } from 'react'
import {
  Save,
  Play,
  Loader2,
  Terminal,
  Edit2,
  Check,
  X,
  GitCommitHorizontal,
  ShieldCheck,
  ShieldOff,
} from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Textarea } from '@/components/ui/textarea'
import { motion, AnimatePresence } from 'framer-motion'
import { useAppStore } from '@/lib/app-store'
import { mystParse } from 'myst-parser'
import { MyST } from 'myst-to-react'
import { Theme, ThemeProvider } from '@myst-theme/providers'
import type { NodeRenderers } from '@myst-theme/providers'
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter'
import { vscDarkPlus } from 'react-syntax-highlighter/dist/esm/styles/prism'
import { toast } from 'sonner'
import { cn } from '@/lib/utils'
import { TauriClient } from '@/lib/tauri-client'
import { CodeBlockEditor } from './CodeBlockEditor'
import { TerminalPanel } from './TerminalPanel'
import './myst-styles.css'

// AST Node Types
interface ASTNode {
  type: string
  value?: string
  lang?: string
  language?: string
  children?: ASTNode[]
  [key: string]: unknown
}

// Extended type for code blocks
interface CodeBlockNode extends ASTNode {
  type: 'code' | 'codeBlock'
  lang?: string
  language?: string
  value?: string
}

// Embedded Block Component - Renders referenced blocks inline
const EmbeddedBlock = ({ blockId }: { blockId: string }) => {
  const [error, setError] = useState<string | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [hasPermission, setHasPermission] = useState(false)

  // Get block from store
  const block = useAppStore((state) => {
    const currentFileId = state.currentFileId
    if (!currentFileId) return null
    return state.getBlock(currentFileId, blockId)
  })

  const currentFileId = useAppStore((state) => state.currentFileId)
  const checkPermission = useAppStore((state) => state.checkPermission)

  useEffect(() => {
    const checkBlockPermission = async () => {
      if (!currentFileId) {
        setError('无法加载：未打开文件')
        setIsLoading(false)
        return
      }

      if (!block) {
        setError('未找到块')
        setIsLoading(false)
        return
      }

      try {
        // Check permission based on block type
        let permissionGranted = false

        if (block.block_type === 'markdown') {
          permissionGranted = await checkPermission(
            currentFileId,
            blockId,
            'markdown.read'
          )
        } else if (block.block_type === 'code') {
          permissionGranted = await checkPermission(
            currentFileId,
            blockId,
            'code.read'
          )
        } else {
          // For other block types, just show them
          permissionGranted = true
        }

        if (!permissionGranted) {
          setError('🚫 无权访问此块')
          setHasPermission(false)
        } else {
          setHasPermission(true)
        }

        setIsLoading(false)
      } catch (err) {
        console.error('Failed to check permission:', err)
        setError(
          '权限检查失败: ' + (err instanceof Error ? err.message : String(err))
        )
        setIsLoading(false)
      }
    }

    checkBlockPermission()
  }, [blockId, currentFileId, block, checkPermission])

  if (isLoading) {
    return (
      <div className="my-4 rounded-lg border border-border/50 bg-muted/10 p-4 text-center">
        <Loader2 className="mx-auto h-5 w-5 animate-spin text-muted-foreground" />
        <p className="mt-2 text-sm text-muted-foreground">加载中...</p>
      </div>
    )
  }

  if (error || !hasPermission) {
    return (
      <div className="my-4 rounded-lg border border-destructive/50 bg-destructive/10 p-4">
        <p className="text-sm text-destructive">{error || '无权访问'}</p>
      </div>
    )
  }

  if (!block) {
    return (
      <div className="my-4 rounded-lg border border-yellow-200 bg-yellow-50 p-4">
        <p className="text-sm text-yellow-800">⚠️ 未找到块: {blockId}</p>
      </div>
    )
  }

  // Render markdown blocks
  if (block.block_type === 'markdown') {
    const contents = block.contents as { content?: string; text?: string }
    const content = contents?.content || contents?.text || ''
    return (
      <div className="my-4 rounded-lg border-l-4 border-blue-500 bg-blue-50/50 p-4">
        <div className="mb-2 flex items-center justify-between">
          <span className="text-xs font-semibold text-blue-800">
            📄 {block.name}
          </span>
          <Badge variant="secondary" className="text-[10px]">
            嵌入块
          </Badge>
        </div>
        <div className="prose prose-sm max-w-none">
          <MySTDocument content={content} />
        </div>
      </div>
    )
  }

  // Render code blocks
  if (block.block_type === 'code') {
    const contents = block.contents as {
      content?: string
      text?: string
      language?: string
    }
    const content = contents?.content || contents?.text || ''
    const language = contents?.language || 'text'
    return (
      <div className="my-4 rounded-lg border-l-4 border-purple-500 bg-purple-50/50 p-4">
        <div className="mb-2 flex items-center justify-between">
          <span className="text-xs font-semibold text-purple-800">
            💻 {block.name}
          </span>
          <Badge variant="secondary" className="text-[10px]">
            嵌入代码块
          </Badge>
        </div>
        <CodeBlockWithRun code={content} language={language} />
      </div>
    )
  }

  // Other block types
  return (
    <div className="my-4 rounded-lg border border-border bg-muted/20 p-4">
      <div className="mb-2 flex items-center justify-between">
        <span className="text-xs font-semibold text-foreground">
          📦 {block.name}
        </span>
        <Badge variant="secondary" className="text-[10px]">
          {block.block_type}
        </Badge>
      </div>
      <p className="text-sm text-muted-foreground">不支持嵌入此类型的块</p>
    </div>
  )
}

// Code Block Component with Run Button
const CodeBlockWithRun = ({
  code,
  language,
}: {
  code: string
  language: string
}) => {
  const [isRunning, setIsRunning] = useState(false)
  const [output, setOutput] = useState<string | null>(null)

  const handleRun = async (e: React.MouseEvent) => {
    e.stopPropagation()
    setIsRunning(true)
    setOutput(null)

    // TODO: Implement real code execution via Terminal Extension
    // This is a mock implementation for demonstration purposes
    // Real implementation should call Tauri backend command to execute code
    setTimeout(() => {
      const mockOutput = `✓ Code executed successfully (Mock)
Language: ${language}
Output:
${code.substring(0, 100)}${code.length > 100 ? '...' : ''}

> Process completed at ${new Date().toLocaleTimeString()}

Note: This is a mock execution. Real code execution will be implemented in Terminal Extension.`
      setOutput(mockOutput)
      setIsRunning(false)
      toast.success('Code executed (mock)')
    }, 1500)
  }

  return (
    <div className="group relative my-4 w-full">
      <div className={cn('overflow-hidden rounded-lg border border-border')}>
        <div className="flex items-center justify-between border-b border-zinc-800 bg-zinc-900 px-4 py-2">
          <span className="font-mono text-xs uppercase text-zinc-300">
            {language}
          </span>
          <Button
            size="sm"
            variant="ghost"
            className="h-7 text-xs text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200"
            onClick={handleRun}
            disabled={isRunning}
          >
            {isRunning ? (
              <>
                <Loader2 className="mr-1.5 h-3 w-3 animate-spin" />
                Running...
              </>
            ) : (
              <>
                <Play className="mr-1.5 h-3 w-3" />
                Run
              </>
            )}
          </Button>
        </div>
        <div className="w-full overflow-x-auto">
          <SyntaxHighlighter
            language={language}
            style={vscDarkPlus}
            customStyle={{
              margin: 0,
              padding: '1rem',
              fontSize: '0.875rem',
              background: '#18181b',
              fontFamily:
                'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", "Microsoft YaHei", "WenQuanYi Micro Hei", monospace',
            }}
            wrapLines={false}
            wrapLongLines={false}
            lineProps={{
              style: {
                background: 'transparent',
              },
            }}
            codeTagProps={{
              style: {
                background: 'transparent',
              },
            }}
          >
            {code}
          </SyntaxHighlighter>
        </div>
        {/* Terminal Output */}
        <AnimatePresence>
          {output && (
            <motion.div
              initial={{ height: 0, opacity: 0 }}
              animate={{ height: 'auto', opacity: 1 }}
              exit={{ height: 0, opacity: 0 }}
              className="border-t border-zinc-800 bg-zinc-950"
            >
              <div className="p-4 font-mono text-sm">
                <div className="mb-2 flex select-none items-center gap-2 text-xs text-zinc-500">
                  <Terminal className="h-3 w-3" />
                  <span>Terminal Output</span>
                </div>
                <pre className="whitespace-pre-wrap font-mono text-xs leading-relaxed text-green-400">
                  {output}
                </pre>
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    </div>
  )
}

// MyST Document Renderer with proper theme and styling
const MySTDocument = ({
  content,
  onContentChange,
  onSave,
}: {
  content: string
  onContentChange?: (newContent: string) => void
  onSave?: () => void | Promise<void>
}) => {
  const codeBlockCounter = useRef(0)
  const [isEditing, setIsEditing] = useState(false)
  const [editContent, setEditContent] = useState(content)
  const [isSaving, setIsSaving] = useState(false)
  const textareaRef = useRef<HTMLTextAreaElement>(null)

  // Extract store methods at component level (React hooks pattern)
  const currentFileId = useAppStore((state) => state.currentFileId)
  const checkPermission = useAppStore((state) => state.checkPermission)
  const selectBlock = useAppStore((state) => state.selectBlock)

  // Parse MyST content into AST
  // According to official guide: use unified processor with mystParser plugin
  const ast = useMemo(() => {
    if (!content || !content.trim()) {
      return null
    }
    try {
      // Reset code block counter on new parse
      codeBlockCounter.current = 0

      // Parse MyST markdown content to AST
      // mystParse handles the unified processor internally
      const parsed = mystParse(content)
      return parsed
    } catch (e) {
      console.error('MyST parse error:', e)
      return null
    }
  }, [content])

  // Update edit content when content prop changes
  useEffect(() => {
    setEditContent(content)
  }, [content])

  // Focus textarea when entering edit mode and auto-resize
  useEffect(() => {
    if (isEditing && textareaRef.current) {
      const textarea = textareaRef.current
      textarea.focus()
      // Move cursor to end instead of selecting all
      setTimeout(() => {
        textarea.setSelectionRange(textarea.value.length, textarea.value.length)
      }, 0)

      // Auto-resize textarea to fit content
      const autoResize = () => {
        textarea.style.height = 'auto'
        textarea.style.height = textarea.scrollHeight + 'px'
      }
      autoResize()

      // Add resize observer
      textarea.addEventListener('input', autoResize)
      return () => textarea.removeEventListener('input', autoResize)
    }
  }, [isEditing])

  // Custom renderers for all node types with proper text content handling
  // IMPORTANT: NodeRenderer only provides `node` and `className` props
  // For container nodes, manually render children via <MyST ast={node.children} />
  // For leaf nodes, use `node.value` directly
  const customRenderers: NodeRenderers = useMemo(
    () => ({
      // Headings (h1-h6)
      heading: ({ node }) => {
        const level = (node.depth as number) || 1
        const Tag = `h${level}` as keyof JSX.IntrinsicElements
        return (
          <Tag>
            <MyST ast={node.children} />
          </Tag>
        )
      },

      // Paragraph
      paragraph: ({ node }) => {
        return (
          <p>
            <MyST ast={node.children} />
          </p>
        )
      },

      // Strong (bold) - CRITICAL for **text**
      strong: ({ node }) => {
        return (
          <strong>
            <MyST ast={node.children} />
          </strong>
        )
      },

      // Emphasis (italic) - for *text*
      emphasis: ({ node }) => {
        return (
          <em>
            <MyST ast={node.children} />
          </em>
        )
      },

      // Text node - CRITICAL: this renders the actual text content
      text: ({ node }) => {
        return <>{node.value}</>
      },

      // List (ul/ol)
      list: ({ node }) => {
        const Tag = node.ordered ? 'ol' : 'ul'
        return (
          <Tag>
            <MyST ast={node.children} />
          </Tag>
        )
      },

      // List item
      listItem: ({ node }) => {
        return (
          <li>
            <MyST ast={node.children} />
          </li>
        )
      },

      // Link - with elf:// protocol support
      link: ({ node }) => {
        const url = node.url as string

        // Check if this is an elf:// block reference
        if (url && url.startsWith('elf://')) {
          const blockId = url.replace('elf://', '')

          const handleClick = async (e: React.MouseEvent) => {
            e.preventDefault()

            if (!currentFileId) {
              toast.error('无法跳转：未打开文件')
              return
            }

            // Check permission before navigating
            try {
              const hasPermission = await checkPermission(
                currentFileId,
                blockId,
                'markdown.read'
              )

              if (!hasPermission) {
                const hasCodePermission = await checkPermission(
                  currentFileId,
                  blockId,
                  'code.read'
                )

                if (!hasCodePermission) {
                  toast.error('您没有读取此块的权限')
                  return
                }
              }

              // Navigate to block
              selectBlock(blockId)
              toast.success('已跳转到引用块')
            } catch (error) {
              console.error('Failed to check permission:', error)
              toast.error('跳转失败')
            }
          }

          return (
            <a
              href="#"
              onClick={handleClick}
              style={{
                color: '#2563eb',
                textDecoration: 'underline',
                cursor: 'pointer',
              }}
              className="elf-block-link"
              title={`跳转到块: ${blockId}`}
            >
              <MyST ast={node.children} />
            </a>
          )
        }

        // Regular link
        return (
          <a
            href={url}
            target="_blank"
            rel="noopener noreferrer"
            style={{ color: '#2563eb', textDecoration: 'underline' }}
          >
            <MyST ast={node.children} />
          </a>
        )
      },

      // Inline code
      inlineCode: ({ node }) => {
        return <code>{node.value}</code>
      },

      // Code block with Run button
      code: ({ node }) => {
        const codeNode = node as CodeBlockNode
        const code = codeNode.value || ''
        const language = codeNode.lang || codeNode.language || 'text'
        codeBlockCounter.current += 1
        return <CodeBlockWithRun code={code} language={language} />
      },

      // Blockquote
      blockquote: ({ node }) => {
        return (
          <blockquote>
            <MyST ast={node.children} />
          </blockquote>
        )
      },

      // Horizontal rule (thematic break)
      thematicBreak: () => {
        return <hr />
      },

      // Break (line break)
      break: () => {
        return <br />
      },

      // MyST Directive - handles :::{embed} syntax
      mystDirective: ({ node }) => {
        const directiveName = node.name as string

        // Handle embed directive
        if (directiveName === 'embed') {
          let blockId = ''

          // First try to get from args (directive argument)
          if (node.args && typeof node.args === 'string') {
            blockId = node.args.trim()
          }
          // Fallback: try to get from children (content body)
          else if (node.children && Array.isArray(node.children)) {
            const firstChild = node.children[0] as ASTNode
            if (firstChild && firstChild.type === 'text' && firstChild.value) {
              blockId = firstChild.value.trim()
            } else if (
              firstChild &&
              firstChild.type === 'paragraph' &&
              firstChild.children
            ) {
              const textNode = firstChild.children[0] as ASTNode
              if (textNode && textNode.value) {
                blockId = textNode.value.trim()
              }
            }
          }

          if (!blockId) {
            return (
              <div className="my-4 rounded-lg border border-yellow-200 bg-yellow-50 p-4">
                <p className="text-sm text-yellow-800">⚠️ 嵌入指令缺少块 ID</p>
              </div>
            )
          }

          // Render embedded block content
          return <EmbeddedBlock blockId={blockId} />
        }

        // Other directives - render children normally
        return (
          <div className="my-4 rounded-lg border border-border bg-muted/20 p-4">
            <div className="mb-2 text-xs font-semibold uppercase text-muted-foreground">
              {directiveName}
            </div>
            <MyST ast={node.children} />
          </div>
        )
      },

      // MyST Directive Error - handles directive parsing errors
      mystDirectiveError: ({ node }) => {
        const directiveName = node.name as string
        const message = node.message as string
        const value = node.value as string

        return (
          <div className="my-4 rounded-lg border-2 border-red-300 bg-red-50 p-4">
            <div className="mb-2 flex items-center gap-2">
              <span className="text-sm font-semibold text-red-800">
                ⚠️ Directive Error: {directiveName}
              </span>
            </div>
            <p className="text-sm text-red-700">{message}</p>
            {value && (
              <pre className="mt-2 rounded bg-red-100 p-2 text-xs text-red-900">
                {value}
              </pre>
            )}
            <p className="mt-2 text-xs text-red-600">
              正确语法: :::{'{'}embed{'}'} block-id
            </p>
          </div>
        )
      },

      // Embed Node - fallback handler
      embed: ({ node }) => {
        const blockId =
          (node.args as string) ||
          (node.value as string) ||
          ((node.children?.[0] as ASTNode)?.value as string) ||
          ''
        if (blockId) {
          return <EmbeddedBlock blockId={blockId} />
        }
        return (
          <div className="my-4 rounded-lg border border-yellow-200 bg-yellow-50 p-4">
            <p className="text-sm text-yellow-800">⚠️ Embed 节点缺少块 ID</p>
          </div>
        )
      },
    }),
    [currentFileId, checkPermission, selectBlock]
  )

  const handleSave = useCallback(async () => {
    // Update content
    if (editContent.trim() !== content.trim()) {
      if (onContentChange) {
        onContentChange(editContent)
      }
    }

    // Save block to database
    if (onSave) {
      setIsSaving(true)
      try {
        await onSave()
        setIsEditing(false)
      } catch (error) {
        console.error('Failed to save block:', error)
        // Keep editing mode open on error
      } finally {
        setIsSaving(false)
      }
    } else {
      setIsEditing(false)
    }
  }, [editContent, content, onContentChange, onSave])

  const handleCancel = useCallback(() => {
    setEditContent(content)
    setIsEditing(false)
  }, [content])

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
      e.preventDefault()
      handleSave()
    } else if (e.key === 'Escape') {
      e.preventDefault()
      handleCancel()
    }
  }

  // Edit mode - show raw MyST markdown with Jupyter-like UI
  if (isEditing) {
    return (
      <div className="group relative my-4 w-full">
        {/* Floating toolbar - appears on focus */}
        <div className="sticky top-4 z-20 mb-3 flex justify-end">
          <motion.div
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            className="flex gap-2 rounded-lg border border-border bg-background/95 p-2 shadow-lg backdrop-blur-sm"
          >
            <div className="flex items-center gap-2 border-r border-border pr-3">
              <Badge variant="secondary" className="h-6 text-xs font-medium">
                <Edit2 className="mr-1 h-3 w-3" />
                Edit
              </Badge>
              <span className="text-xs text-muted-foreground">
                MyST Markdown
              </span>
            </div>
            <Button
              size="sm"
              variant="ghost"
              className="h-7 text-xs"
              onClick={handleCancel}
            >
              <X className="mr-1.5 h-3.5 w-3.5" />
              Cancel
            </Button>
            <Button
              size="sm"
              variant="default"
              className="h-7 text-xs font-medium"
              onClick={handleSave}
              disabled={isSaving}
            >
              {isSaving ? (
                <>
                  <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />
                  Saving...
                </>
              ) : (
                <>
                  <Check className="mr-1.5 h-3.5 w-3.5" />
                  Save
                </>
              )}
            </Button>
          </motion.div>
        </div>

        {/* Textarea with preview-like styling */}
        <div className="rounded-lg border border-primary/30 bg-background shadow-sm transition-all hover:border-primary/50">
          <Textarea
            ref={textareaRef}
            value={editContent}
            onChange={(e) => setEditContent(e.target.value)}
            onKeyDown={handleKeyDown}
            className={cn(
              'min-h-[500px] resize-none overflow-hidden',
              'border-0 bg-transparent',
              'focus-visible:ring-0 focus-visible:ring-offset-0',
              'font-mono text-sm leading-relaxed',
              'p-8'
            )}
            placeholder="Start typing your MyST Markdown content here...

Examples:
# Heading 1
## Heading 2

**Bold text** and *italic text*

- Bullet point 1
- Bullet point 2

```python
print('Hello, World!')
```"
          />
        </div>
      </div>
    )
  }

  // Preview mode - render document with proper MyST styling
  const astWithChildren = ast as ASTNode | null
  if (
    !astWithChildren ||
    !astWithChildren.children ||
    astWithChildren.children.length === 0
  ) {
    return (
      <div className={cn('myst-rendered', 'group relative w-full')}>
        <div
          className="relative my-4 cursor-text rounded-lg border border-dashed border-border/50 bg-muted/10 p-16 text-center transition-all hover:border-border hover:bg-muted/20"
          onDoubleClick={() => setIsEditing(true)}
        >
          <div className="text-muted-foreground">
            <Edit2 className="mx-auto mb-3 h-10 w-10 opacity-20" />
            <p className="mb-2 text-base font-medium">Empty Block</p>
            <p className="text-sm">Double-click to start editing</p>
          </div>
        </div>
      </div>
    )
  }

  // Render document using MyST pattern with custom CSS (text-display-css-spec.md)
  // 1. Apply .myst-rendered class to the outer container
  // 2. ThemeProvider wraps MyST component and provides theme context
  // 3. MyST component renders AST as HTML elements (strong, em, h1, etc.)
  // 4. Custom CSS in myst-styles.css provides all styling
  // 5. Custom renderers extend functionality (e.g., code blocks with Run button)
  return (
    <div className={cn('myst-rendered', 'group relative w-full')}>
      {/* Document Content - Double-click to edit (Jupyter-like) */}
      <div
        onDoubleClick={() => setIsEditing(true)}
        className="cursor-text rounded-lg p-8 transition-all hover:bg-muted/5"
      >
        {/* ThemeProvider provides context for MyST components */}
        <ThemeProvider
          theme={Theme.light}
          setTheme={() => {}}
          renderers={customRenderers}
        >
          {/* MyST renders the AST as actual HTML elements (h1, p, strong, etc.) */}
          <MyST ast={ast as any} />
        </ThemeProvider>
      </div>

      {/* Subtle hint - only shows on hover */}
      <div className="mt-4 text-center text-xs text-muted-foreground opacity-0 transition-opacity group-hover:opacity-60">
        💡 Double-click to edit
      </div>
    </div>
  )
}

// Task Toolbar: commit action + commit protect toggle for task blocks
const TaskToolbar = ({ blockId }: { blockId: string }) => {
  const { currentFileId, commitTask, saveFile, getBlocks } = useAppStore()
  const [isCommitting, setIsCommitting] = useState(false)
  const [commitProtect, setCommitProtect] = useState(false)
  const [isToggling, setIsToggling] = useState(false)

  // Get linked repo paths from directory blocks with source=linked
  const linkedRepoPaths = useMemo(() => {
    if (!currentFileId) return []
    const blocks = getBlocks(currentFileId)
    return blocks
      .filter(
        (b) =>
          b.block_type === 'directory' &&
          (b.contents as Record<string, unknown>)?.source === 'linked'
      )
      .map(
        (b) =>
          (b.metadata as Record<string, unknown>)?.external_root_path as string
      )
      .filter(Boolean)
  }, [currentFileId, getBlocks])

  // Check initial hooks status on mount
  useEffect(() => {
    if (!currentFileId || linkedRepoPaths.length === 0) return
    TauriClient.task
      .isHooksActive(currentFileId, linkedRepoPaths[0])
      .then(setCommitProtect)
      .catch(() => setCommitProtect(false))
  }, [currentFileId, linkedRepoPaths])

  const handleCommit = useCallback(async () => {
    if (!currentFileId) return

    setIsCommitting(true)
    try {
      const result = await commitTask(currentFileId, blockId)
      await saveFile(currentFileId)
      toast.success(
        `Committed to ${result.branch_name} (${result.commit_hash.slice(0, 7)})`
      )
    } catch {
      // Error toast shown by store
    } finally {
      setIsCommitting(false)
    }
  }, [currentFileId, blockId, commitTask, saveFile])

  const handleToggleProtect = useCallback(async () => {
    if (!currentFileId || linkedRepoPaths.length === 0) return

    setIsToggling(true)
    try {
      for (const repoPath of linkedRepoPaths) {
        if (commitProtect) {
          await TauriClient.task.removeHooksForRepo(currentFileId, repoPath)
        } else {
          await TauriClient.task.injectHooksForRepo(currentFileId, repoPath)
        }
      }
      setCommitProtect(!commitProtect)
      toast.info(
        commitProtect ? 'Commit protect disabled' : 'Commit protect enabled'
      )
    } catch {
      toast.error('Failed to toggle commit protect')
    } finally {
      setIsToggling(false)
    }
  }, [currentFileId, linkedRepoPaths, commitProtect])

  return (
    <div className="flex items-center gap-2 border-t border-border pt-4">
      <Button
        size="sm"
        variant="outline"
        onClick={handleCommit}
        disabled={isCommitting}
      >
        <GitCommitHorizontal className="mr-1.5 h-3.5 w-3.5" />
        {isCommitting ? 'Committing...' : 'Commit'}
      </Button>
      {linkedRepoPaths.length > 0 && (
        <Button
          size="sm"
          variant={commitProtect ? 'default' : 'ghost'}
          onClick={handleToggleProtect}
          disabled={isToggling}
          title={
            commitProtect
              ? 'Commit protect ON — external direct commits blocked'
              : 'Commit protect OFF — external direct commits allowed'
          }
        >
          {commitProtect ? (
            <ShieldCheck className="h-3.5 w-3.5" />
          ) : (
            <ShieldOff className="h-3.5 w-3.5" />
          )}
        </Button>
      )}
    </div>
  )
}

// Main EditorCanvas Component
export const EditorCanvas = () => {
  const {
    currentFileId,
    selectedBlockId,
    updateBlock,
    saveFile,
    loadEvents,
    checkPermission,
    fetchBlock,
    getBlock,
  } = useAppStore()
  const [isSaving, setIsSaving] = useState(false)
  const [documentContent, setDocumentContent] = useState<string>('')
  const [showTerminal, setShowTerminal] = useState(false)
  const [fetchError, setFetchError] = useState<string | null>(null)
  const [isFetching, setIsFetching] = useState(false)
  const [lastDocBlockId, setLastDocBlockId] = useState<string | null>(null)

  // Update lastDocBlockId when a real document is selected
  useEffect(() => {
    const block = selectedBlockId
      ? getBlock(currentFileId || '', selectedBlockId)
      : null
    if (
      block &&
      (block.block_type === 'markdown' ||
        block.block_type === 'code' ||
        block.block_type === 'task')
    ) {
      setLastDocBlockId(block.block_id)
    }
  }, [selectedBlockId, currentFileId, getBlock])
  // Subscribe directly to block changes from Zustand store
  // This ensures UI refreshes when restoreToEvent updates blocks
  const selectedBlock = useAppStore((state) => {
    if (!currentFileId || !selectedBlockId) return null
    const fileState = state.files.get(currentFileId)
    return fileState?.blocks.find((b) => b.block_id === selectedBlockId) || null
  })

  // Attempt to fetch block if selected but not loaded (e.g. permission issues or deep link)
  useEffect(() => {
    setFetchError(null)

    if (currentFileId && selectedBlockId && !selectedBlock) {
      setIsFetching(true)
      fetchBlock(currentFileId, selectedBlockId)
        .then((block) => {
          if (!block) {
            setFetchError('Block not found or access denied.')
          }
        })
        .finally(() => {
          setIsFetching(false)
        })
    }
  }, [currentFileId, selectedBlockId, selectedBlock, fetchBlock])

  // Load block content when selectedBlock changes
  useEffect(() => {
    // ONLY update document content for non-terminal blocks
    // This allows the editor to 'stick' to its doc content when switching to terminals
    if (selectedBlock && selectedBlock.block_type !== 'terminal') {
      // Clear error if block is successfully loaded
      setFetchError(null)
      setIsFetching(false)

      const contents = selectedBlock.contents as {
        markdown?: string
        text?: string
      }
      if (selectedBlock.block_type === 'code') {
        setDocumentContent(contents?.text || '')
      } else if (
        selectedBlock.block_type === 'markdown' ||
        selectedBlock.block_type === 'task'
      ) {
        setDocumentContent(contents?.markdown || contents?.text || '')
      } else {
        setDocumentContent(contents?.markdown || contents?.text || '')
      }
    } else if (!selectedBlockId) {
      setDocumentContent('')
    }
    // If it's a terminal block, we DO NOT update documentContent,
    // preserving the content of the previous document.
  }, [selectedBlock, selectedBlockId])

  // Handle save block only (called from editor)
  const handleBlockSave = useCallback(async () => {
    // Editor save button: Only saves editing state locally, does NOT generate event
    // Event generation happens only when Ctrl+S is pressed (handleSave)
    // This just confirms the edit and exits edit mode
    toast.success('Editing saved - Press Ctrl+S to persist changes')
  }, [])

  // Handle save file action (Ctrl+S - top-level save button)
  const handleSave = useCallback(async () => {
    if (!currentFileId || !selectedBlockId || !selectedBlock) {
      toast.error('No block selected')
      return
    }

    setIsSaving(true)
    try {
      // Get current block content
      let currentContent = ''
      if (
        selectedBlock.block_type === 'markdown' ||
        selectedBlock.block_type === 'task'
      ) {
        const contents = selectedBlock.contents as {
          markdown?: string
          text?: string
        }
        currentContent = contents?.markdown || contents?.text || ''
      } else if (selectedBlock.block_type === 'code') {
        const contents = selectedBlock.contents as { text?: string }
        currentContent = contents?.text || ''
      }

      // Check if content has changed
      const hasChanges = documentContent.trim() !== currentContent.trim()

      if (hasChanges) {
        // Content changed: save block first (generates event)
        const capId =
          selectedBlock.block_type === 'code'
            ? 'code.write'
            : selectedBlock.block_type === 'task'
              ? 'task.write'
              : 'markdown.write'
        const hasPermission = await checkPermission(
          currentFileId,
          selectedBlockId,
          capId
        )

        if (!hasPermission) {
          toast.error('You do not have permission to edit this block.')
          setIsSaving(false)
          return
        }

        await updateBlock(
          currentFileId,
          selectedBlockId,
          documentContent,
          selectedBlock.block_type
        )
      }

      // Always save file to disk (persists all events)
      await saveFile(currentFileId)

      // Reload events to show any new events in Timeline
      if (hasChanges) {
        await loadEvents(currentFileId)
      }

      // Different message based on whether content changed
      if (hasChanges) {
        toast.success('Changes saved and file persisted to disk')
      } else {
        toast.success('File saved to disk')
      }
    } catch (error) {
      console.error('Failed to save:', error)
      // Error toast is already shown by app-store methods
    } finally {
      setIsSaving(false)
    }
  }, [
    currentFileId,
    selectedBlockId,
    selectedBlock,
    documentContent,
    checkPermission,
    updateBlock,
    saveFile,
    loadEvents,
  ])

  // Handle content change from sub-editors
  const handleContentChange = useCallback((newContent: string) => {
    setDocumentContent(newContent)
  }, [])

  // Keyboard shortcut for save (Ctrl+S / Cmd+S)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === 's') {
        e.preventDefault()
        handleSave()
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [handleSave])

  const renderEditor = () => {
    // Effective block for rendering in the canvas
    let blockToRender = selectedBlock

    // If terminal is selected, try to show the previous document instead
    if (
      selectedBlock?.block_type === 'terminal' &&
      lastDocBlockId &&
      currentFileId
    ) {
      const lastDoc = getBlock(currentFileId, lastDocBlockId)
      if (lastDoc) {
        blockToRender = lastDoc
      }
    }

    if (!blockToRender) {
      if (fetchError) {
        return (
          <div className="py-16 text-center text-muted-foreground">
            <p className="mb-2 text-destructive">Error loading block</p>
            <p className="text-sm">{fetchError}</p>
          </div>
        )
      }
      if (selectedBlockId || isFetching) {
        return (
          <div className="flex flex-col items-center justify-center py-16 text-muted-foreground">
            <Loader2 className="mb-2 h-8 w-8 animate-spin opacity-50" />
            <p>Loading block...</p>
          </div>
        )
      }
      return (
        <div className="py-16 text-center text-muted-foreground">
          <p>No block selected</p>
          <p className="mt-2 text-sm">
            Select a block from the file panel to start editing
          </p>
        </div>
      )
    }

    if (blockToRender.block_type === 'code') {
      const language = blockToRender.name.split('.').pop() || 'text'
      return (
        <CodeBlockEditor
          content={documentContent}
          language={language}
          onContentChange={handleContentChange}
          onSave={handleBlockSave}
        />
      )
    }

    if (blockToRender.block_type === 'markdown') {
      return (
        <MySTDocument
          content={documentContent}
          onContentChange={handleContentChange}
          onSave={handleBlockSave}
        />
      )
    }

    if (blockToRender.block_type === 'task') {
      return (
        <div className="space-y-4">
          <MySTDocument
            content={documentContent}
            onContentChange={handleContentChange}
            onSave={handleBlockSave}
          />
          <TaskToolbar blockId={blockToRender.block_id} />
        </div>
      )
    }

    if (blockToRender.block_type === 'terminal') {
      // If we got here, it means there was no 'lastDocBlockId' to fall back to.
      // Just show a cleaner message than the generic unsupported one.
      return (
        <div className="py-16 text-center text-muted-foreground">
          <Terminal className="mx-auto mb-4 h-12 w-12 opacity-20" />
          <p>Terminal session active</p>
          <p className="mt-2 text-sm italic">
            Select a document in the panel to enable editing here.
          </p>
        </div>
      )
    }

    return (
      <div className="py-16 text-center text-muted-foreground">
        <p>Unsupported Block Type: {blockToRender.block_type}</p>
        <p className="mt-2 text-sm">This block cannot be rendered yet.</p>
      </div>
    )
  }

  return (
    <div className="flex h-full flex-col">
      <ScrollArea className="flex-1">
        <main className="w-full min-w-0 bg-background p-4 md:p-6 lg:p-8">
          <div className="mx-auto w-full min-w-0 max-w-4xl pb-32">
            {/* Save Button */}
            <div className="mb-6 flex items-center justify-end">
              <motion.div
                whileHover={{ scale: 1.02 }}
                whileTap={{ scale: 0.98 }}
              >
                <Button
                  size="sm"
                  className="bg-foreground px-4 text-sm font-medium text-background shadow-sm hover:bg-foreground/90"
                  onClick={(e) => {
                    e.stopPropagation()
                    handleSave()
                  }}
                  disabled={isSaving}
                >
                  <Save className="mr-2 h-3.5 w-3.5" />
                  {isSaving ? 'Saving...' : 'Save (Ctrl+S)'}
                </Button>
              </motion.div>
            </div>

            <div className="w-full min-w-0 space-y-6">{renderEditor()}</div>
          </div>
        </main>
      </ScrollArea>

      {/* Terminal Toggle Button */}
      {!showTerminal && (
        <div className="flex justify-center border-t border-zinc-800/50 bg-zinc-950/50 p-1.5 backdrop-blur-sm">
          <Button
            size="sm"
            variant="ghost"
            onClick={() => setShowTerminal(true)}
            disabled={!currentFileId}
            className="h-7 text-xs font-medium text-zinc-400 transition-colors hover:bg-zinc-800/50 hover:text-cyan-400 disabled:opacity-50"
          >
            <Terminal className="mr-1.5 h-3.5 w-3.5" />
            Open Terminal
          </Button>
        </div>
      )}

      {/* Terminal Panel */}
      {showTerminal && currentFileId && (
        <TerminalPanel
          fileId={currentFileId}
          onClose={() => setShowTerminal(false)}
        />
      )}
    </div>
  )
}
