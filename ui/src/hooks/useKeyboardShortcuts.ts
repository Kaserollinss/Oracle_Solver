import { useEffect } from 'react'
import { useUiStore } from '../store/useUiStore'

export function useKeyboardShortcuts() {
  const togglePlayer = useUiStore((s) => s.togglePlayer)
  const setActivePlayer = useUiStore((s) => s.setActivePlayer)
  const setActiveStreet = useUiStore((s) => s.setActiveStreet)
  const cycleViewMode = useUiStore((s) => s.cycleViewMode)
  const pinHand = useUiStore((s) => s.pinHand)

  useEffect(() => {
    function handler(e: KeyboardEvent) {
      if (
        e.target instanceof HTMLInputElement ||
        e.target instanceof HTMLSelectElement ||
        e.target instanceof HTMLTextAreaElement
      ) {
        return
      }

      switch (e.key.toLowerCase()) {
        case 'q':
          setActivePlayer('IP')
          break
        case 'w':
          setActivePlayer('OOP')
          break
        case '1':
          setActiveStreet('Flop')
          break
        case '2':
          setActiveStreet('Turn')
          break
        case '3':
          setActiveStreet('River')
          break
        case ' ':
          e.preventDefault()
          cycleViewMode()
          break
        case 'escape':
          pinHand(null)
          break
      }
    }

    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [togglePlayer, setActivePlayer, setActiveStreet, cycleViewMode, pinHand])
}
