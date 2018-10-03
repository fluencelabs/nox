export interface SessionSummary {
    status: any
    invokedTxsCount: number
    lastTxCounter: number
}

export interface BroadcastTxSyncResponse {
    code: number
    data: string
    log: string
    hash: string
}