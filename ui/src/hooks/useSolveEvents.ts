import { useEffect } from 'react'
import { listen } from '@tauri-apps/api/event'
import { useSolverStore } from '../store/useSolverStore'
import type { SolveProgressDto } from '../bindings'

export function useSolveEvents() {
  const updateProgress = useSolverStore((s) => s.updateProgress)
  const setSolveComplete = useSolverStore((s) => s.setSolveComplete)

  useEffect(() => {
    const unlistenProgress = listen<SolveProgressDto>('solve-progress', (e) => {
      console.debug('[event] solve-progress:', e.payload)
      updateProgress(e.payload)
    })

    const unlistenComplete = listen<SolveProgressDto>('solve-complete', (e) => {
      console.debug('[event] solve-complete:', e.payload)
      setSolveComplete(e.payload)
    })

    return () => {
      unlistenProgress.then((f) => f())
      unlistenComplete.then((f) => f())
    }
  }, [updateProgress, setSolveComplete])
}
