---
description: '@race-foundation/borsh'
---

# 🧱 Serialization with Borsh

## **Introduction**

**Borsh** (Binary Object Representation Serializer for Hashing) is a standardized, deterministic serialization format. In the context of the RACE Protocol, it is the cornerstone for ensuring that data structures are consistently represented across different parts of the system — from your web client to the on-chain game logic.

### **Why is this important?**

* **Determinism**: Serializing the same JavaScript object will always produce the exact same sequence of bytes. This is critical for cryptographic operations like hashing and signing, where even a minor difference would result in a completely different output.
* **Compactness**: The binary format is compact, making it efficient for network transmission and on-chain storage, which helps reduce transaction fees.
* **Interoperability**: It defines a clear specification for how data is structured. This is essential for the reliable communication between the off-chain client (your application) and the on-chain game logic (the WebAssembly handler).

The `@race-foundation/borsh` package is a powerful implementation of this standard, designed with developer experience in mind. It uses TypeScript decorators and helper functions to make the process of defining serializable data structures intuitive and straightforward.

***

## **Defining Schemas**

To make a class serializable, you must first define its "schema." This tells the library how to convert each property into its binary format. This is done by decorating the class properties with `@field` and using helper functions for complex types.

*   **Primitives:** Use the `@field` decorator with a string literal for basic types.

    * `'u8'`, `'u16'`, `'u32'`: Unsigned integers. These map to JavaScript `number`.
    * `'u64'`, `'usize'`: Large unsigned integers. These must be handled as `bigint` in JavaScript.
    * `'bool'`: A boolean value, serialized as a single byte (`0` or `1`).
    * `'string'`: A UTF-8 string, prefixed with its length as a `u32`.
    * `'u8-array'`: A dynamic array of bytes (`Uint8Array`), also prefixed with its length as a `u32`.

    ```typescript
    import { field } from '@race-foundation/borsh';

    class Player {
      @field('u8')
      level!: number;

      @field('u64')
      experience!: bigint;

      @field('string')
      name!: string;
    }
    ```
*   **Fixed-Size Byte Arrays:** For arrays with a known, fixed length (like public keys or hashes), use the `@field` decorator with a number representing the byte length.

    ```typescript
    import { field } from '@race-foundation/borsh';

    class CryptoKeys {
      @field(32) // A 32-byte public key
      publicKey!: Uint8Array;
    }
    ```
*   **Dynamic Arrays:** For arrays of any other type where the length can vary, use the `array()` helper function. It takes the type definition of the array's elements as its argument.

    ```typescript
    import { field, array, struct } from '@race-foundation/borsh';

    class Item {
        @field('u32')
        id!: number;
    }

    class Inventory {
      // An array of primitive numbers
      @field(array('u8'))
      itemQuantities!: number[];

      // An array of other serializable objects (structs)
      @field(array(struct(Item)))
      items!: Item[];
    }
    ```
*   **Structs (Nested Objects):** To nest one serializable object within another, use the `struct()` helper, passing the class constructor of the nested object.

    ```typescript
    import { field, struct } from '@race-foundation/borsh';

    class Position {
      @field('u32')
      x!: number;
      @field('u32')
      y!: number;
    }

    class GameObject {
      @field(struct(Position))
      position!: Position;
    }
    ```
*   **Enums (Variants):** Enums allow you to serialize one of several different object shapes under a common abstract type. This is perfect for game events or states that can have multiple forms.

    1. Define an `abstract` base class.
    2. For each variant, create a class that extends the base class.
    3. Decorate each variant class with `@variant(index)`, where `index` is a unique `u8` number (0-255) identifying that variant.

    ```typescript
    import { field, variant, enums } from '@race-foundation/borsh';

    // 1. Define the abstract base class
    abstract class GameEvent {}

    // 2. Create and decorate each variant
    @variant(0)
    class PlayerMove extends GameEvent {
      @field('u32') x!: number;
      @field('u32') y!: number;
    }

    @variant(1)
    class PlayerAttack extends GameEvent {
      @field('u64') targetId!: bigint;
    }

    // In another class, use the `enums()` helper with the base class
    class Action {
        @field(enums(GameEvent))
        event!: GameEvent;
    }
    ```
*   **Options (Optional Fields):** For fields that might be `undefined` or `null`, wrap their type with the `option()` helper. This adds a 1-byte prefix (`0` for none, `1` for some) to the serialized data.

    ```typescript
    import { field, option } from '@race-foundation/borsh';

    class PlayerProfile {
      @field(option('string'))
      nickname?: string;
    }

    const player1 = new PlayerProfile(); // nickname is undefined
    const player2 = new PlayerProfile();
    player2.nickname = 'Racer';
    ```
*   **Maps:** To serialize `Map` objects, use the `map()` helper, specifying the key type and value type.

    ```typescript
    import { field, map } from '@race-foundation/borsh';

    class Scoreboard {
      @field(map('string', 'u32'))
      scores!: Map<string, number>;
    }
    ```

***

## **Using `serialize` and `deserialize`**

Once your schemas are defined, converting object instances to and from byte arrays is a simple two-function process.

* `serialize(object: any): Uint8Array`: Takes an instance of a schema-defined class and returns its `Uint8Array` byte representation.
* `deserialize<T>(class: Ctor<T> | EnumClass<T>, data: Uint8Array): T`: Takes the class constructor (which holds the schema information) and a `Uint8Array`, and returns a new, hydrated instance of that class.

### **Complete Example:**

```typescript
import { field, serialize, deserialize } from '@race-foundation/borsh';

// 1. Define the schema
class Vec2 {
  @field('u32')
  x!: number;

  @field('u32')
  y!: number;

  // A constructor that accepts fields is recommended for easy instantiation
  constructor(fields: { x: number, y: number }) {
    Object.assign(this, fields);
  }
}

// 2. Create an instance
const myVector = new Vec2({ x: 10, y: 20 });

// 3. Serialize the object
const serializedData: Uint8Array = serialize(myVector);
// -> Uint8Array([10, 0, 0, 0, 20, 0, 0, 0])

// 4. Deserialize the data
const deserializedVector: Vec2 = deserialize(Vec2, serializedData);

console.log(deserializedVector.x); // Outputs: 10
console.log(deserializedVector.y); // Outputs: 20
console.log(deserializedVector instanceof Vec2); // Outputs: true
```

> **For more advanced examples, including nested structs, enums, and arrays, refer to the tests in the `@race-foundation/borsh` package, specifically `packages/borsh/tests/serialize.spec.ts`.**

***

## **Command-Line Tool**

For quick tests, scripting, or debugging, the `@race-foundation/borsh` package provides a handy command-line tool, `borsh-serialize`, to serialize data without writing any code.

You can run it directly with `npx`.

**Usage:**

```bash
npx borsh-serialize [-s|-b|-u8|-u16|-u32|-u64 VALUE]...
```

**Options:**

* `-s STRING`: Appends a string.
* `-u8 INT`: Appends an integer as a `u8`.
* `-u16 INT`: Appends an integer as a `u16`.
* `-u32 INT`: Appends an integer as a `u32`.
* `-u64 INT`: Appends an integer as a `u64`.
* `-b BOOL`: Appends a boolean (`true` or `false`).

**Example:**

Let's serialize a string "abc", followed by the boolean true, followed by the number 100 as a u64.

```bash
npx borsh-serialize -s abc -b true -u64 100
```

**Output:**

```bash
[3,0,0,0,97,98,99,1,100,0,0,0,0,0,0,0]
```

* **`[3,0,0,0]`**: The `u32` length prefix for the string "abc".
* **`[97,98,99]`**: The UTF-8 bytes for "a", "b", and "c".
* **`[1]`**: The `u8` representation of true.
* **`[100,0,0,0,0,0,0,0]`**: The little-endian `u64` representation of 100.
