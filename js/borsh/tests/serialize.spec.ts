import { deserialize, extend, field, option, serialize, struct, variant, enums, array, map } from '../src/index';
import { assert } from 'chai';
import { ExtendOptions, IExtendReader, IExtendWriter } from '../src/types';
import { writeU64 } from '../src/writer';
import { readU64 } from '../src/reader';

describe('Test serialize', () => {
  it('U8', () => {
    class C {
      @field('u8')
      x!: number;
      @field('u8')
      y!: number;
      constructor(fields: { x: number; y: number }) {
        Object.assign(this, fields);
      }
    }
    const c = new C({ x: 1, y: 2 });
    const bs = serialize(c);
    assert.deepEqual(bs, Uint8Array.from([1, 2]));
    const c0 = deserialize(C, bs);
    assert.deepEqual(c, c0);
  });

  it('U32', () => {
    class C {
      @field('u32')
      x!: number;
      @field('u32')
      y!: number;
      constructor(fields: { x: number; y: number }) {
        Object.assign(this, fields);
      }
    }
    const c = new C({ x: 1, y: 2 });
    const bs = serialize(c);
    assert.deepEqual(bs, Uint8Array.from([1, 0, 0, 0, 2, 0, 0, 0]));
    const c0 = deserialize(C, bs);
    assert.deepEqual(c, c0);
  });

  it('U64', () => {
    class C {
      @field('u64')
      x!: bigint;
      @field('u64')
      y!: bigint;
      constructor(fields: { x: bigint; y: bigint }) {
        Object.assign(this, fields);
      }
    }
    const c = new C({
      x: 12345678901n,
      y: 312312312312n,
    });
    const bs = serialize(c);
    assert.deepEqual(bs, Uint8Array.from([53, 28, 220, 223, 2, 0, 0, 0, 248, 177, 67, 183, 72, 0, 0, 0]));
    const c0 = deserialize(C, bs);
    assert.deepEqual(c, c0);
  });

  it('String', () => {
    class C {
      @field('string')
      x!: string;
      @field('string')
      y!: string;
      constructor(fields: { x: string; y: string }) {
        Object.assign(this, fields);
      }
    }
    const c = new C({ x: 'foo', y: 'barbaz' });
    const bs = serialize(c);
    assert.deepEqual(bs, Uint8Array.from([3, 0, 0, 0, 102, 111, 111, 6, 0, 0, 0, 98, 97, 114, 98, 97, 122]));
    const c0 = deserialize(C, bs);
    assert.deepEqual(c, c0);
  });

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
    assert.deepEqual(bs, Uint8Array.from([1, 2, 3, 4]));
    const c0 = deserialize(C, bs);
    assert.deepEqual(c, c0);
  });

  it('Uint8Array', () => {
    class C {
      @field('u8-array')
      x!: Uint8Array;
      constructor(fields: { x: Uint8Array }) {
        Object.assign(this, fields);
      }
    }
    const c = new C({ x: Uint8Array.of(1, 2, 3, 4) });
    const bs = serialize(c);
    assert.deepEqual(bs, Uint8Array.from([4, 0, 0, 0, 1, 2, 3, 4]));
    const c0 = deserialize(C, bs);
    assert.deepEqual(c, c0);
  });

  it('Array of structs', () => {
    class A {
      @field('u8')
      x!: number;
      constructor(fields: { x: number }) {
        Object.assign(this, fields);
      }
    }

    class C {
      @field(array(struct(A)))
      x!: A[];
      constructor(fields: { x: A[] }) {
        Object.assign(this, fields);
      }
    }
    const c = new C({
      x: [new A({ x: 1 }), new A({ x: 2 })],
    });
    const bs = serialize(c);
    assert.deepEqual(bs, Uint8Array.from([2, 0, 0, 0, 1, 2]));
    const c0 = deserialize(C, bs);
    assert.deepEqual(c, c0);
  });

  it('Map', () => {
    class C {
      @field(map('u8', 'string'))
      x!: Map<number, string>;
      constructor(fields: { x: Map<number, string> }) {
        Object.assign(this, fields);
      }
    }
    const c = new C({
      x: new Map([
        [1, 'a'],
        [2, 'b'],
      ]),
    });
    const bs = serialize(c);
    assert.deepEqual(bs, Uint8Array.from([2, 0, 0, 0, 1, 1, 0, 0, 0, 97, 2, 1, 0, 0, 0, 98]));
    const c0 = deserialize(C, bs);
    assert.deepEqual(c, c0);
  });

  it('Option', () => {
    class C {
      @field(option('u8'))
      x: number | undefined;
      @field(option('string'))
      y: string | undefined;
      constructor(fields: { x?: number; y?: string }) {
        this.x = fields.x;
        this.y = fields.y;
      }
    }
    let c = new C({
      x: 127,
      y: 'hello',
    });
    let bs = serialize(c);
    assert.deepEqual(bs, Uint8Array.from([1, 127, 1, 5, 0, 0, 0, 104, 101, 108, 108, 111]));
    let c0 = deserialize(C, bs);
    assert.deepEqual(c, c0);

    c = new C({
      y: 'hello',
    });
    bs = serialize(c);
    assert.deepEqual(bs, Uint8Array.from([0, 1, 5, 0, 0, 0, 104, 101, 108, 108, 111]));
    c0 = deserialize(C, bs);
    assert.deepEqual(c, c0);
  });

  it('Extend', () => {
    class DateWriter implements IExtendWriter<Date> {
      write(value: Date, buf: Uint8Array, offset: number) {
        writeU64(BigInt(value.getTime()), buf, offset);
      }
    }
    class DateReader implements IExtendReader<Date> {
      read(buf: Uint8Array, offset: number): Date {
        const v = readU64(buf, offset);
        return new Date(Number(v));
      }
    }
    const dateOptions: ExtendOptions<Date> = {
      size: 8,
      writer: new DateWriter(),
      reader: new DateReader(),
    };

    class C {
      @field('u32')
      x: number;
      @field(extend(dateOptions))
      y: Date;
      @field('u32')
      z: number;
      constructor(fields: { y: Date, x: number, z: number }) {
        this.x = fields.x;
        this.y = fields.y;
        this.z = fields.z;
      }
    }
    let c = new C({
      x: 11111111,
      y: new Date(1640966400000),
      z: 22222222,
    });
    let bs = serialize(c);
    const c0 = deserialize(C, bs);
    assert.deepEqual(c, c0);
  });

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

      @field(array(struct(Y)))
      y!: Y[];

      constructor(fields: { x: X; y: Y[] }) {
        Object.assign(this, fields);
      }
    }
    const c = new C({
      x: new X({ x: 1 }),
      y: [new Y({ x: 'foo' }), new Y({ x: 'bar' })],
    });
    const bs = serialize(c);
    assert.deepEqual(bs, Uint8Array.from([1, 2, 0, 0, 0, 3, 0, 0, 0, 102, 111, 111, 3, 0, 0, 0, 98, 97, 114]));
    const c0 = deserialize(C, bs);
    assert.deepEqual(c, c0);
  });

  it('Enum', () => {
    abstract class A {}

    @variant(0)
        class B extends A {
          @field('u8')
          x!: number;
          constructor(fields: { x: number }) {
            super();
            Object.assign(this, fields);
          }
        }

    @variant(1)
        class C extends A {
          @field('u64')
          x!: bigint;
          constructor(fields: { x: bigint }) {
            super();
            Object.assign(this, fields);
          }
        }

    const c = new C({ x: 2n });
    const bs = serialize(c);
    assert.deepEqual(bs, Uint8Array.from([1, 2, 0, 0, 0, 0, 0, 0, 0]));
    const c0 = deserialize(A, bs);
    assert.deepEqual(c0, c);

    class D {
      @field('u8')
      x: number;

      @field(enums(A))
      y: A;

      constructor(fields: { x: number; y: A }) {
        this.x = fields.x;
        this.y = fields.y;
      }
    }

    const d = new D({ x: 1, y: new B({ x: 127 }) });
    const bs0 = serialize(d);
    assert.deepEqual(bs0, Uint8Array.from([1, 0, 127]));
    const d0 = deserialize(D, bs0);
    assert.deepEqual(d0, d);
  });
});
