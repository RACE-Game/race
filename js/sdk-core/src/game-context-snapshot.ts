import { GameContext, GameStatus, INode, NodeStatus } from './game-context'

export class NodeSnapshot {
    readonly addr: string
    readonly status: NodeStatus

    constructor(o: INode) {
        this.addr = o.addr
        this.status = o.status
    }
}

export class GameContextSnapshot {
    readonly gameAddr: string
    readonly accessVersion: bigint
    readonly settleVersion: bigint
    readonly status: GameStatus
    readonly nodes: NodeSnapshot[]

    constructor(context: GameContext) {
        this.gameAddr = context.spec.gameAddr
        this.accessVersion = context.accessVersion
        this.settleVersion = context.settleVersion
        this.status = context.status
        this.nodes = context.nodes.map((n: INode) => new NodeSnapshot(n))
    }
}
