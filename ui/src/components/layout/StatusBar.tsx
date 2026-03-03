import { useSolverStore } from '@/store/useSolverStore'

export function StatusBar() {
  const status = useSolverStore((s) => s.status)
  const progress = useSolverStore((s) => s.progress)
  const treeInfo = useSolverStore((s) => s.treeInfo)

  const statusLabel =
    typeof status === 'string' ? status : 'Error'

  const isActive = statusLabel === 'Solving'

  return (
    <div className="h-7 bg-(--s1) border-t border-(--border) flex items-center px-4 gap-6 shrink-0 z-60">
      <div className="text-[10px] text-(--muted) font-mono flex items-center gap-1.5">
        <span
          className={`w-1.5 h-1.5 rounded-full inline-block ${isActive ? 'bg-(--green) animate-pulse' : 'bg-(--muted)'}`}
        />
        <span>
          {statusLabel}
          {progress && ` — ${progress.iteration.toLocaleString()} iterations`}
        </span>
      </div>

      {treeInfo && (
        <div className="text-[10px] text-(--muted) font-mono">
          Nodes: <em className="not-italic text-(--green)">{treeInfo.total_nodes.toLocaleString()}</em>
        </div>
      )}

      {progress && (
        <div className="text-[10px] text-(--muted) font-mono">
          Exploitability:{' '}
          <em className="not-italic text-(--green)">{progress.exploitability.toFixed(4)} bb</em>
        </div>
      )}

      <div className="text-[10px] text-(--muted) font-mono ml-auto">
        CFR+ • Apple Silicon
      </div>
    </div>
  )
}
