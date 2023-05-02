import { field, array, getSchema } from '../src/index';
import { assert } from 'chai';

// export class Geo {
//   @field(array(Point))
//   points: Point[]
// }

describe('Test decorations', () => {
  it('Test field decorations', () => {
    class Point {
      @field('u8')
      x!: number;
      @field('u8')
      y!: number;
      constructor(fields: {x: number, y: number}) {
        this.x = fields.x;
        this.y = fields.y;
      }
    };
    const p = new Point({x: 1, y: 2});
    console.log(p);
    assert.deepEqual(
      getSchema(Point),
      ['u8', 'u8']
    );
  })
})
