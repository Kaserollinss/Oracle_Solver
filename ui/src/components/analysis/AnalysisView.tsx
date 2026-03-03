import { useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { TreeNavigator } from './TreeNavigator'
import { StrategyMatrix } from './StrategyMatrix'
import { DetailPanel } from './DetailPanel'
import { useSolverStore } from '@/store/useSolverStore'
import type { StrategyMatrixDto } from '@/bindings'

export function AnalysisView() {
  const status = useSolverStore((s) => s.status)
  const currentNodeId = useSolverStore((s) => s.currentNodeId)
  const setStrategyMatrix = useSolverStore((s) => s.setStrategyMatrix)

  const isResolved = status === 'Converged' || status === 'Stopped'

  // Fetch strategy matrix when node changes or solve completes
  useEffect(() => {
    if (!isResolved) return

    async function fetchMatrix() {
      try {
        const matrix = await invoke<StrategyMatrixDto>('get_strategy_matrix', {
          nodeId: currentNodeId,
        })
        console.debug('[analysis] matrix fetched for node', currentNodeId, ':', matrix.cells.length, 'cells')
        setStrategyMatrix(matrix)
      } catch (e) {
        console.error('[analysis] matrix fetch error:', e)
      }
    }

    fetchMatrix()
  }, [currentNodeId, isResolved, setStrategyMatrix])

  return (
    <div className="flex flex-1 overflow-hidden">
      <TreeNavigator />
      <StrategyMatrix />
      <DetailPanel />
    </div>
  )
}
