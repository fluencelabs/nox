export interface SessionSummary {
    status: Status
    invokedTxsCount: number
    lastTxCounter: number
}

interface Status {
    Active?: any
}

export function isActive(summary: SessionSummary): boolean {
    return summary.status.Active !== undefined
}
