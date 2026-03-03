import { create } from 'zustand'
import type {
  SolveConfig,
  SolveProgressDto,
  TreeInfoDto,
  StrategyMatrixDto,
  SolveStatus,
} from '../bindings'

interface SolverState {
  // Config
  board: string
  pot: number
  stacks: number
  betSizes: number[]
  raiseSizes: number[]
  maxRaises: number
  allinThreshold: number
  maxIterations: number
  threshold: number
  checkEvery: number
  timeCap: number

  // Solve state
  status: SolveStatus
  progress: SolveProgressDto | null
  treeInfo: TreeInfoDto | null
  progressHistory: SolveProgressDto[]

  // Results
  currentNodeId: number
  strategyMatrix: StrategyMatrixDto | null

  // Actions
  setBoard: (board: string) => void
  setPot: (pot: number) => void
  setStacks: (stacks: number) => void
  setBetSizes: (sizes: number[]) => void
  setRaiseSizes: (sizes: number[]) => void
  setMaxRaises: (n: number) => void
  setMaxIterations: (n: number) => void
  setThreshold: (t: number) => void
  setCheckEvery: (n: number) => void
  setTimeCap: (n: number) => void

  updateProgress: (p: SolveProgressDto) => void
  setSolveComplete: (p: SolveProgressDto) => void
  setTreeInfo: (info: TreeInfoDto) => void
  setStatus: (s: SolveStatus) => void
  setCurrentNodeId: (id: number) => void
  setStrategyMatrix: (m: StrategyMatrixDto | null) => void
  resetSolve: () => void

  getSolveConfig: () => SolveConfig
}

export const useSolverStore = create<SolverState>((set, get) => ({
  // Config defaults
  board: '',
  pot: 6.0,
  stacks: 97.0,
  betSizes: [0.33, 0.75],
  raiseSizes: [1.0],
  maxRaises: 1,
  allinThreshold: 0.1,
  maxIterations: 1000,
  threshold: 0.5,
  checkEvery: 100,
  timeCap: 300,

  // Solve state
  status: 'Idle' as SolveStatus,
  progress: null,
  treeInfo: null,
  progressHistory: [],

  // Results
  currentNodeId: 0,
  strategyMatrix: null,

  // Config setters
  setBoard: (board) => {
    console.debug('[store] setBoard:', board)
    set({ board })
  },
  setPot: (pot) => set({ pot }),
  setStacks: (stacks) => set({ stacks }),
  setBetSizes: (betSizes) => set({ betSizes }),
  setRaiseSizes: (raiseSizes) => set({ raiseSizes }),
  setMaxRaises: (maxRaises) => set({ maxRaises }),
  setMaxIterations: (maxIterations) => set({ maxIterations }),
  setThreshold: (threshold) => set({ threshold }),
  setCheckEvery: (checkEvery) => set({ checkEvery }),
  setTimeCap: (timeCap) => set({ timeCap }),

  // Solve state setters
  updateProgress: (p) => {
    console.debug('[store] progress:', p.iteration, 'exploit:', p.exploitability.toFixed(4))
    set((state) => ({
      progress: p,
      status: p.status,
      progressHistory: [...state.progressHistory, p],
    }))
  },
  setSolveComplete: (p) => {
    console.debug('[store] solve complete:', p.status)
    set({ progress: p, status: p.status })
  },
  setTreeInfo: (treeInfo) => set({ treeInfo }),
  setStatus: (status) => {
    console.debug('[store] status:', status)
    set({ status })
  },
  setCurrentNodeId: (currentNodeId) => {
    console.debug('[store] currentNodeId:', currentNodeId)
    set({ currentNodeId })
  },
  setStrategyMatrix: (strategyMatrix) => set({ strategyMatrix }),
  resetSolve: () =>
    set({
      status: 'Idle' as SolveStatus,
      progress: null,
      treeInfo: null,
      progressHistory: [],
      currentNodeId: 0,
      strategyMatrix: null,
    }),

  getSolveConfig: () => {
    const s = get()
    return {
      game: {
        board: s.board,
        pot: s.pot,
        stacks: s.stacks,
        bet_sizes: s.betSizes,
        raise_sizes: s.raiseSizes,
        max_raises_per_street: s.maxRaises,
        allin_threshold: s.allinThreshold,
      },
      max_iterations: s.maxIterations,
      threshold: s.threshold,
      check_every: s.checkEvery,
      time_cap_secs: s.timeCap,
    }
  },
}))
