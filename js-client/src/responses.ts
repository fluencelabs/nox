export interface SessionSummary {
    status: string
    invokedTxsCount: number
    lastTxCounter: number
}

export interface BroadcastTxSyncResponse {
    code: number
    data: string
    log: string
    hash: string
}