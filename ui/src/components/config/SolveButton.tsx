import { invoke } from '@tauri-apps/api/core'
import { useSolverStore } from '@/store/useSolverStore'
import type { TreeInfoDto } from '@/bindings'

export function SolveButton() {
  const status = useSolverStore((s) => s.status)
  const progress = useSolverStore((s) => s.progress)
  const board = useSolverStore((s) => s.board)
  const getSolveConfig = useSolverStore((s) => s.getSolveConfig)
  const setTreeInfo = useSolverStore((s) => s.setTreeInfo)
  const setStatus = useSolverStore((s) => s.setStatus)
  const resetSolve = useSolverStore((s) => s.resetSolve)

  const isSolving = status === 'Solving' || status === 'Building'
  const boardLen = board.length / 2
  const canSolve = !isSolving && (boardLen === 3 || boardLen === 4 || boardLen === 5)

  async function handleSolve() {
    if (isSolving) {
      // Stop
      console.debug('[solve] stopping...')
      try {
        await invoke('stop_solve')
      } catch (e) {
        console.error('[solve] stop error:', e)
      }
      return
    }

    // Start
    resetSolve()
    setStatus('Building')
    console.debug('[solve] starting with config:', getSolveConfig())

    try {
      const treeInfo = await invoke<TreeInfoDto>('start_solve', {
        config: getSolveConfig(),
      })
      console.debug('[solve] tree built:', treeInfo)
      setTreeInfo(treeInfo)
    } catch (e) {
      console.error('[solve] start error:', e)
      setStatus({ Error: { message: String(e) } } as never)
    }
  }

  const progressPct = progress
    ? Math.min(100, (progress.iteration / Math.max(1, progress.max_iterations)) * 100)
    : 0

  return (
    <div className="p-3.5 bg-(--s1) border-t border-(--border) shrink-0 shadow-[0_-4px_12px_rgba(0,0,0,0.2)] z-10">
      <button
        className={`w-full py-2.5 rounded-md font-semibold text-xs tracking-wide cursor-pointer transition-all ${
          isSolving
            ? 'bg-transparent border border-(--accent) text-(--accent)'
            : canSolve
              ? 'bg-gradient-to-br from-(--accent) to-(--accent3) text-black border-none hover:opacity-90 hover:-translate-y-px'
              : 'bg-(--s3) text-(--muted) border border-(--border2) cursor-not-allowed'
        }`}
        onClick={handleSolve}
        disabled={!canSolve && !isSolving}
      >
        {isSolving ? '⟳ Stop Solver' : '▶ Run Solver'}
      </button>

      {isSolving && progress && (
        <div className="mt-2">
          <div className="h-[3px] bg-(--s3) rounded-sm overflow-hidden">
            <div
              className="h-full bg-gradient-to-r from-(--accent) to-(--accent2) transition-[width] duration-500"
              style={{ width: `${progressPct}%` }}
            />
          </div>
          <div className="flex justify-between mt-1 text-[9px] text-(--muted) font-mono">
            <span>
              iter {progress.iteration.toLocaleString()} — {progress.exploitability.toFixed(4)} bb
            </span>
            <span>{Math.round(progressPct)}%</span>
          </div>
        </div>
      )}
    </div>
  )
}
