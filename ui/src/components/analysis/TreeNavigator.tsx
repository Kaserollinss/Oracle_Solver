import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { useSolverStore } from '@/store/useSolverStore'
import { cn } from '@/lib/utils'
import type { TreeNodeDto } from '@/bindings'

export function TreeNavigator() {
  const status = useSolverStore((s) => s.status)
  const currentNodeId = useSolverStore((s) => s.currentNodeId)
  const setCurrentNodeId = useSolverStore((s) => s.setCurrentNodeId)
  const [treeNodes, setTreeNodes] = useState<TreeNodeDto[]>([])

  const isResolved = status === 'Converged' || status === 'Stopped'

  useEffect(() => {
    if (!isResolved) return

    async function fetchTree() {
      try {
        const nodes = await invoke<TreeNodeDto[]>('get_tree_structure')
        console.debug('[tree] fetched', nodes.length, 'nodes')
        setTreeNodes(nodes)
      } catch (e) {
        console.error('[tree] fetch error:', e)
      }
    }

    fetchTree()
  }, [isResolved])

  // Build a simple hierarchy: root + first level of children
  const rootNode = treeNodes.find((n) => n.id === 0)

  return (
    <div className="w-60 bg-(--s1) border-r border-(--border) p-3.5 overflow-y-auto shrink-0">
      <div className="text-[9px] uppercase tracking-[2px] text-(--muted) mb-3">Game Tree</div>

      {!isResolved ? (
        <div className="text-[11px] text-(--muted)">Run a solve to see the tree.</div>
      ) : treeNodes.length === 0 ? (
        <div className="text-[11px] text-(--muted)">Loading tree...</div>
      ) : (
        <TreeNodeList
          nodes={treeNodes}
          depth={0}
          currentNodeId={currentNodeId}
          onSelect={setCurrentNodeId}
          rootId={rootNode?.id ?? 0}
        />
      )}
    </div>
  )
}

function TreeNodeList({
  nodes,
  depth,
  currentNodeId,
  onSelect,
  rootId,
}: {
  nodes: TreeNodeDto[]
  depth: number
  currentNodeId: number
  onSelect: (id: number) => void
  rootId: number
}) {
  // At depth 0, show root
  if (depth === 0) {
    const root = nodes.find((n) => n.id === rootId)
    if (!root) return null

    return (
      <>
        <TreeNodeItem
          node={root}
          isActive={currentNodeId === root.id}
          onClick={() => onSelect(root.id)}
        />
        <div className="pl-3.5 border-l border-dashed border-(--border2) ml-1.5 mb-1">
          {root.children
            .map((childId) => nodes.find((n) => n.id === childId))
            .filter((n): n is TreeNodeDto => n !== undefined && n.node_type === 'decision')
            .map((child) => (
              <div key={child.id}>
                <TreeNodeItem
                  node={child}
                  isActive={currentNodeId === child.id}
                  onClick={() => onSelect(child.id)}
                />
                {/* Show one more level */}
                <div className="pl-3.5 border-l border-dashed border-(--border2) ml-1.5 mb-1">
                  {child.children
                    .map((cid) => nodes.find((n) => n.id === cid))
                    .filter((n): n is TreeNodeDto => n !== undefined && n.node_type === 'decision')
                    .map((grandchild) => (
                      <TreeNodeItem
                        key={grandchild.id}
                        node={grandchild}
                        isActive={currentNodeId === grandchild.id}
                        onClick={() => onSelect(grandchild.id)}
                      />
                    ))}
                </div>
              </div>
            ))}
        </div>
      </>
    )
  }

  return null
}

function TreeNodeItem({
  node,
  isActive,
  onClick,
}: {
  node: TreeNodeDto
  isActive: boolean
  onClick: () => void
}) {
  return (
    <div
      className={cn(
        'py-1.5 px-2.5 rounded cursor-pointer text-[11px] transition-all flex items-center gap-1.5 mb-0.5 border border-transparent',
        isActive
          ? 'bg-[rgba(192,132,252,0.1)] border-[rgba(192,132,252,0.2)] text-(--accent)'
          : 'text-(--text2) hover:bg-(--s2) hover:text-(--text)'
      )}
      onClick={onClick}
    >
      <span className="font-mono text-[10px]">
        {node.player ?? 'CH'}
      </span>
      <span className="text-[10px] text-(--muted) ml-auto">
        {node.street ?? ''}
      </span>
    </div>
  )
}
