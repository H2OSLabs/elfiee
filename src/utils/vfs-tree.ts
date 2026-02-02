import type { Block } from '@/bindings'

export interface DirectoryEntry {
  id: string
  type: string
  source: string
  external_path?: string
  updated_at: string
}

export interface VfsNode {
  path: string
  name: string
  type: 'file' | 'directory'
  blockId: string | null
  blockType?: string
  source: 'outline' | 'linked'
  children: VfsNode[]
  isExpanded?: boolean
}

/**
 * 将 Directory Block 的扁平 entries 转换为嵌套树结构
 *
 * 算法：
 * 1. 遍历所有 entries，按路径深度排序
 * 2. 使用 Map 缓存已创建的节点（path → VfsNode）
 * 3. 对每个 entry，找到父节点并插入
 */
export function buildTreeFromEntries(
  entries: Record<string, DirectoryEntry>,
  blocks: Block[]
): VfsNode[] {
  const nodeMap = new Map<string, VfsNode>()
  const roots: VfsNode[] = []

  // 1. 获取所有路径并按深度排序（浅到深），确保处理子节点前父节点已处理
  // 性能优化：缓存深度计算，避免在排序循环中重复 split
  const pathWithDepths = Object.keys(entries).map((path) => ({
    path,
    depth: path.split('/').filter(Boolean).length,
  }))

  pathWithDepths.sort((a, b) => a.depth - b.depth)
  const sortedPaths = pathWithDepths.map((pd) => pd.path)

  for (const path of sortedPaths) {
    const entry = entries[path]
    const segments = path.split('/').filter(Boolean)
    const name = segments[segments.length - 1] || path

    // 尝试获取关联 Block 的类型
    const relatedBlock =
      entry.type === 'file'
        ? blocks.find((b) => b.block_id === entry.id)
        : undefined

    const node: VfsNode = {
      path,
      name,
      type: entry.type as 'file' | 'directory',
      blockId: relatedBlock ? entry.id : null,
      blockType: relatedBlock?.block_type,
      source: entry.source as 'outline' | 'linked',
      children: [],
      isExpanded: false,
    }

    // Normalize key: 去掉尾随斜杠，使 "agents/" 和 "agents" 匹配
    const normalizedPath = path.endsWith('/') ? path.slice(0, -1) : path
    nodeMap.set(normalizedPath, node)

    // 2. 确定层级关系
    if (segments.length <= 1) {
      // 顶级节点
      roots.push(node)
    } else {
      // 寻找父路径 (例如 "src/utils/math.rs" -> "src/utils")
      const parentPath = segments.slice(0, -1).join('/')
      const parent = nodeMap.get(parentPath)

      if (parent) {
        parent.children.push(node)
      } else {
        // 容错：如果父节点不存在于 entries 中（非规范数据），作为根节点处理
        roots.push(node)
      }
    }
  }

  // 3. 对每一层进行排序（可选：文件夹在前，文件名按字母序）
  const sortNodes = (nodes: VfsNode[]) => {
    nodes.sort((a, b) => {
      if (a.type !== b.type) {
        return a.type === 'directory' ? -1 : 1
      }
      return a.name.localeCompare(b.name)
    })
    nodes.forEach((node) => {
      if (node.children.length > 0) {
        sortNodes(node.children)
      }
    })
  }

  sortNodes(roots)

  return roots
}
