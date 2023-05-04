import { extend, field, option, serialize, struct, variant, vec } from '../src/index';
import { assert } from 'chai';
import { ExtendOptions, IExtendWriter } from '../src/types';
import { writeU32, writeU64 } from '../src/writer';

describe('Test serialize', () => {
  it('U8', () => {
    class C {
      @field('u8')
      x!: number;
      @field('u8')
      y!: number;
      constructor(fields: { x: number, y: number }) {
        Object.assign(this, fields);
      }
    };
    const c = new C({ x: 1, y: 2 });
    const bs = serialize(c);
    assert.deepEqual(bs, Uint8Array.from([1, 2]));
  })

  it('U64', () => {
    class C {
      @field('u64')
      x!: bigint;
      @field('u64')
      y!: bigint;
      constructor(fields: { x: bigint, y: bigint }) {
        Object.assign(this, fields);
      }
    };
    const c = new C({
      x: BigInt(12345678901),
      y: BigInt(312312312312)
    });
    const bs = serialize(c);
    assert.deepEqual(bs, Uint8Array.from([53, 28, 220, 223, 2, 0, 0, 0, 248, 177, 67, 183, 72, 0, 0, 0]));
  })

  it('String', () => {
    class C {
      @field('string')
      x!: string;
      @field('string')
      y!: string;
      constructor(fields: { x: string, y: string }) {
        Object.assign(this, fields);
      }
    }
    const c = new C({ x: 'foo', y: 'barbaz' });
    const bs = serialize(c);
    assert.deepEqual(bs, Uint8Array.from([3, 0, 0, 0, 102, 111, 111, 6, 0, 0, 0, 98, 97, 114, 98, 97, 122]))
  })

  it('Fixed sized byte array', () => {
    class C {
      @field(4)
      x!: Uint8Array;
      constructor(fields: { x: Uint8Array }) {
        Object.assign(this, fields);
      }
    }
    const c = new C({ x: Uint8Array.of(1, 2, 3, 4) });
    const bs = serialize(c);
    assert.deepEqual(bs, Uint8Array.from([1, 2, 3, 4]))
  })

  it('Dynamic sized byte array with vec', () => {
    class C {
      @field(vec('u8'))
      x!: Uint8Array;
      constructor(fields: { x: Uint8Array }) {
        Object.assign(this, fields);
      }
    }
    const c = new C({ x: Uint8Array.of(1, 2, 3, 4) });
    const bs = serialize(c);
    assert.deepEqual(bs, Uint8Array.from([4, 0, 0, 0, 1, 2, 3, 4]))
  })

  it('Option', () => {
    class C {
      @field(option('u8'))
      x: number | undefined;
      @field(option('string'))
      y: string | undefined;
      constructor(fields: { x?: number, y?: string }) {
        Object.assign(this, fields);
      }
    };
    let c = new C({
      x: 127,
      y: "hello"
    });
    let bs = serialize(c);
    assert.deepEqual(bs, Uint8Array.from([1, 127, 1, 5, 0, 0, 0, 104, 101, 108, 108, 111]));
    c = new C({
      y: "hello"
    });
    bs = serialize(c);
    assert.deepEqual(bs, Uint8Array.from([0, 1, 5, 0, 0, 0, 104, 101, 108, 108, 111]))
  })

  it('Extend', () => {
    class DateWriter implements IExtendWriter<Date> {
      write(value: Date, buf: Uint8Array, offset: number) {
        writeU64(BigInt(value.getTime()), buf, offset);
      }
    }
    const dateOptions: ExtendOptions<Date> = {
      size: 8,
      writer: new DateWriter()
    };

    class C {
      @field(extend(dateOptions))
      x!: Date;
      constructor(fields: { x: Date }) {
        Object.assign(this, fields);
      }
    };
    let c = new C({
      x: new Date('2022-01-01T00:00:00')
    });
    let bs = serialize(c);
    assert.deepEqual(bs, Uint8Array.from([0, 40, 56, 17, 126, 1, 0, 0]));
  })

  it('Struct', () => {
    class X {
      @field('u8')
      x!: number;
      constructor(fields: { x: number }) {
        Object.assign(this, fields);
      }
    }

    class Y {
      @field('string')
      x!: string;
      constructor(fields: { x: string }) {
        Object.assign(this, fields);
      }
    }

    class C {
      @field(struct(X))
      x!: X;

      @field(vec(struct(Y)))
      y!: Y[];

      constructor(fields: { x: X, y: Y[] }) {
        Object.assign(this, fields);
      }
    }
    const c = new C({
      x: new X({ x: 1 }),
      y: [
        new Y({ x: "foo" }),
        new Y({ x: "bar" }),
      ]
    });
    const bs = serialize(c);
    assert.deepEqual(bs, Uint8Array.from([1, 2, 0, 0, 0, 3, 0, 0, 0, 102, 111, 111, 3, 0, 0, 0, 98, 97, 114]))})

  it('Enum', () => {
    abstract class A {}

    @variant(0)
    class B extends A {
      @field('u8')
      x!: number;
      constructor(fields: { x: number }) {
        super()
        Object.assign(this, fields);
      }
    }

    @variant(1)
    class C extends A{
      @field('u64')
      x!: bigint;
      constructor(fields: { x: bigint }) {
        super()
        Object.assign(this, fields);
      }
    }

    const c = new C({ x: BigInt(1) });
    const bs = serialize(c);
    assert.deepEqual(bs, Uint8Array.from([1, 1, 0, 0, 0, 0, 0, 0, 0]))
  })
})
