import { assert } from 'chai';
import { ActionTimeout, Ask, Assign, Effect, Release, Reveal, Settle, SettleAdd, SettleSub } from '../src/effect';
import { serialize } from '@race-foundation/borsh';
import { ShuffledList } from '../src/random-state';
import { NoEnoughPlayers } from '../src/error';

describe('Test effect serialization', () => {
  it('serialization', () => {
    let effect = new Effect({
      actionTimeout: new ActionTimeout({ playerAddr: 'alice', timeout: 100n }),
      waitTimeout: 200n,
      startGame: true,
      stopGame: true,
      cancelDispatch: true,
      timestamp: 300_000n,
      currRandomId: 1,
      currDecisionId: 1,
      playersCount: 4,
      serversCount: 4,
      asks: [
        new Ask({
          playerAddr: 'bob',
        }),
      ],
      assigns: [
        new Assign({
          randomId: 5,
          playerAddr: 'bob',
          indexes: [0, 1, 2],
        }),
      ],
      reveals: [
        new Reveal({
          randomId: 6,
          indexes: [0, 1, 2],
        }),
      ],
      releases: [
        new Release({
          decisionId: 7,
        }),
      ],
      initRandomStates: [
        new ShuffledList({
          options: ['a', 'b'],
        }),
      ],
      revealed: new Map([[22, new Map([[11, 'B']])]]),
      answered: new Map([[33, 'A']]),
      settles: [
        new Settle({
          addr: 'alice',
          op: new SettleAdd({ amount: 200n }),
        }),
        new Settle({
          addr: 'bob',
          op: new SettleSub({ amount: 200n }),
        }),
      ],
      handlerState: Uint8Array.of(1, 2, 3, 4),
      error: new NoEnoughPlayers({}),
      allowExit: true,
    });
    const data = serialize(effect);
    const expected = Uint8Array.from([
      1, 5, 0, 0, 0, 97, 108, 105, 99, 101, 100, 0, 0, 0, 0, 0, 0, 0, 1, 200, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 224, 147, 4,
      0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 4, 0, 4, 0, 1, 0, 0, 0, 3, 0, 0, 0, 98, 111, 98, 1,
      0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 98, 111, 98, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0,
      0, 2, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0,
      0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 97, 1,
      0, 0, 0, 98, 1, 0, 0, 0, 22, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 11, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 66, 1, 0, 0, 0,
      33, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 65, 2, 0, 0, 0, 5, 0, 0, 0, 97, 108, 105, 99, 101, 0, 200, 0, 0, 0, 0, 0, 0,
      0, 3, 0, 0, 0, 98, 111, 98, 1, 200, 0, 0, 0, 0, 0, 0, 0, 1, 4, 0, 0, 0, 1, 2, 3, 4, 1, 1, 1,
    ]);
    assert.deepEqual(data, expected);
  });
});
