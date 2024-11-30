import { field } from '@race-foundation/borsh'

export class Message {
    @field('string')
    sender!: string
    @field('string')
    content!: string
    constructor(fields: any) {
        Object.assign(this, fields)
    }
}
