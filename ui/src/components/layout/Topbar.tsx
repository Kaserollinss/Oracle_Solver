import { useSolverStore } from '@/store/useSolverStore'

export function Topbar() {
  const status = useSolverStore((s) => s.status)
  const progress = useSolverStore((s) => s.progress)

  const statusLabel =
    typeof status === 'string'
      ? status.toUpperCase()
      : 'Error' in status
        ? 'ERROR'
        : 'UNKNOWN'

  const exploitText = progress
    ? `${progress.exploitability.toFixed(4)} bb`
    : '—'

  const isResolved = statusLabel === 'CONVERGED' || statusLabel === 'STOPPED'

  return (
    <div className="h-11 bg-(--s1) border-b border-(--border) flex items-center px-4 gap-2.5 shrink-0">
      <div className="flex items-center gap-1.5 text-xs text-(--muted)">
        <span>Sessions</span>
        <span className="text-(--muted)">›</span>
        <span className="text-(--text) font-medium">New Session</span>
      </div>

      <div className="w-px h-4 bg-(--border2)" />

      <div className="ml-auto flex items-center gap-1.5">
        {isResolved && (
          <div className="text-[10px] font-mono px-2 py-0.5 rounded bg-[rgba(74,222,128,0.1)] text-(--green) border border-[rgba(74,222,128,0.2)]">
            {statusLabel}
          </div>
        )}
        {progress && (
          <div className="text-[10px] font-mono px-2 py-0.5 rounded bg-[rgba(192,132,252,0.1)] text-(--accent) border border-[rgba(192,132,252,0.2)]">
            {exploitText}
          </div>
        )}
      </div>
    </div>
  )
}
