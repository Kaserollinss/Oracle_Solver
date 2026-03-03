import { BoardSetup } from './BoardSetup'
import { GameContext } from './GameContext'
import { BetSizing } from './BetSizing'
import { EngineSettings } from './EngineSettings'
import { SolveButton } from './SolveButton'

export function ConfigRail() {
  return (
    <div className="w-60 bg-(--s1) border-r border-(--border) flex flex-col shrink-0">
      <div className="flex-1 overflow-y-auto pb-4">
        <BoardSetup />
        <GameContext />
        <BetSizing />
        <EngineSettings />
      </div>
      <SolveButton />
    </div>
  )
}
