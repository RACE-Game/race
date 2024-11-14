import {
  ArrayFieldType,
  Field,
  FieldKey,
  FieldType,
  OptionFieldType,
  StructFieldType,
  ExtendOptions,
  ExtendFieldType,
  Ctor,
  isPrimitiveType,
  hasExtendReader,
  hasExtendWriter,
  EnumFieldType,
  EnumClass,
  MapFieldType,
} from './types';
import { BinaryWriter } from './writer';
import { BinaryReader } from './reader';
import {
  invalidByteArrayLength,
  extendedWriterNotFound,
  extendedReaderNotFound,
  invalidEnumField,
  invalidCtor,
} from './errors';

class DeserializeError extends Error {
  cause: Error;
  path: string[];
  obj: any | undefined;

  constructor(path: string[], cause: Error, obj: any) {
    super('Deserialize failed');
    this.cause = cause;
    this.path = path;
    this.obj = obj;
    Object.setPrototypeOf(this, DeserializeError.prototype);
  }
}

class SerializeError extends Error {
  cause: Error;
  path: string[];
  fieldType: FieldType;
  value: any;

  constructor(path: string[], cause: Error, fieldType: FieldType, value: any) {
    super('Serialize failed');
    this.cause = cause;
    this.path = path;
    this.fieldType = fieldType;
    this.value = value;
    Object.setPrototypeOf(this, SerializeError.prototype);
  }
}

function addSchemaField(prototype: any, key: FieldKey, fieldType: FieldType) {
  let fields: Field[] = prototype.__schema_fields || [];
  fields.push([key, fieldType]);
  prototype.__schema_fields = fields;
}

function getSchemaFields(prototype: any): Field[] {
  return prototype.__schema_fields || [];
}

function addSchemaVariant<T>(ctor: Ctor<T>, variant: number) {
  ctor.prototype.__schema_variant = variant;
  const superClass = Object.getPrototypeOf(ctor.prototype).constructor;
  let enumVariants = superClass.prototype.__schema_enum_variants || [];
  enumVariants.push(ctor);
  superClass.prototype.__schema_enum_variants = enumVariants;
}

function getSchemaVariant(prototype: any): number | undefined {
  return prototype.__schema_variant;
}

function isVariantObject(obj: any): boolean {
  return getSchemaVariant(Object.getPrototypeOf(obj)) !== undefined;
}

function isEnumClass<T>(obj: any): obj is EnumClass<T> {
  return getSchemaEnumVariants(obj.prototype) !== undefined;
}

function getSchemaEnumVariants(prototype: any): any[] | undefined {
  return prototype.__schema_enum_variants;
}

function serializeValue(path: string[], value: any, fieldType: FieldType, writer: BinaryWriter) {
  try {
    if (isPrimitiveType(fieldType)) {
      if (fieldType === 'u8') {
        writer.writeU8(value);
      } else if (fieldType === 'u16') {
        writer.writeU16(value);
      } else if (fieldType === 'u32') {
        writer.writeU32(value);
      } else if (fieldType === 'u64') {
        writer.writeU64(value);
      } else if (fieldType === 'usize') {
        writer.writeU64(BigInt(value));
      } else if (fieldType === 'bool') {
        writer.writeBool(value);
      } else if (fieldType === 'string') {
        writer.writeString(value);
      } else if (fieldType === 'u8-array') {
        writer.writeU32(value.length);
        for (let i = 0; i < value.length; i++) {
          writer.writeU8(value[i]);
        }
      }
    } else if (typeof fieldType === 'number') {
      if (value.length !== fieldType) {
        invalidByteArrayLength(path, fieldType, value.length);
      }
      writer.writeByteArray(value);
    } else {
      const { kind, value: v } = fieldType;
      if (kind === 'option') {
        if (value === undefined || value === null) {
          writer.writeU8(0);
        } else {
          writer.writeU8(1);
          serializeValue([...path, '<OptionValue>'], value, v, writer);
        }
      } else if (kind === 'array') {
        writer.writeU32(value.length);
        for (let i = 0; i < value.length; i++) {
          serializeValue([...path, `<Array[${i}/${value.length}]>`], value[i], v, writer);
        }
      } else if (kind === 'struct') {
        serializeStruct(path, value, writer);
      } else if (kind === 'enum') {
        serializeEnum(path, value, writer);
      } else if (kind === 'map') {
        writer.writeU32(value.size);
        const [keyType, valueType] = v;
        for (const [k, v] of value) {
          serializeValue([...path, `<Map[key]>`], k, keyType, writer);
          serializeValue([...path, `<Map[value]>`], v, valueType, writer);
        }
      } else if (kind === 'extend') {
        if (hasExtendWriter(v)) {
          writer.writeExtended(value, v);
        } else {
          extendedWriterNotFound(path);
        }
      }
    }
  } catch (err: any) {
    if (err instanceof SerializeError) {
      throw err;
    } else {
      throw new SerializeError(path, err, fieldType, value);
    }
  }
}

function deserializeValue(path: string[], fieldType: FieldType, reader: BinaryReader): any {
  try {
    if (isPrimitiveType(fieldType)) {
      if (fieldType === 'u8') {
        return reader.readU8();
      } else if (fieldType === 'u16') {
        return reader.readU16();
      } else if (fieldType === 'u32') {
        return reader.readU32();
      } else if (fieldType === 'u64') {
        return reader.readU64();
      } else if (fieldType === 'bool') {
        return reader.readBool();
      } else if (fieldType === 'usize') {
        return Number(reader.readU64());
      } else if (fieldType === 'string') {
        return reader.readString();
      } else if (fieldType === 'u8-array') {
        let arr = [];
        const length = reader.readU32();
        for (let i = 0; i < length; i++) {
          let n = reader.readU8();
          arr.push(n);
        }
        return Uint8Array.from(arr);
      }
    } else if (typeof fieldType === 'number') {
      return reader.readByteArray(fieldType);
    } else {
      const { kind, value } = fieldType;
      if (kind === 'array') {
        let arr = [];
        const length = reader.readU32();
        for (let i = 0; i < length; i++) {
          let v = deserializeValue([...path, `<Array[${i}/${length}]>`], value, reader);
          arr.push(v);
        }
        return arr;
      } else if (kind === 'option') {
        let opt = reader.readBool();
        if (opt) {
          const v = deserializeValue([...path, '<OptionValue>'], value, reader);
          return v;
        } else {
          return undefined;
        }
      } else if (kind === 'struct') {
        return deserializeStruct(path, value, reader);
      } else if (kind === 'enum') {
        return deserializeEnum(path, value, reader);
      } else if (kind === 'map') {
        const length = reader.readU32();
        const [keyType, valueType] = value;
        const m = new Map();
        for (let i = 0; i < length; i++) {
          let k = deserializeValue([...path, `<Map[key]>`], keyType, reader);
          let v = deserializeValue([...path, `<Map[value]>`], valueType, reader);
          m.set(k, v);
        }
        return m;
      } else if (kind === 'extend') {
        if (hasExtendReader(value)) {
          return reader.readExtended(value);
        } else {
          extendedReaderNotFound(path);
        }
      }
    }
  } catch (err: any) {
    if (err instanceof DeserializeError) {
      throw err;
    } else {
      throw new DeserializeError(path, err, undefined);
    }
  }
}

function serializeField(path: string[], obj: any, field: Field, writer: BinaryWriter) {
  const [key, fieldType] = field;
  const k = key.toString();
  const value = obj[k];
  serializeValue([...path, k], value, fieldType, writer);
}

function deserializeField(path: string[], obj: Record<string, any>, field: Field, reader: BinaryReader): Object {
  const [key, fieldType] = field;
  const k = key.toString();
  const value = deserializeValue([...path, k], fieldType, reader);
  obj[k] = value;
  return obj;
}

function serializeEnum<T>(path: string[], obj: T, writer: BinaryWriter) {
  const prototype = Object.getPrototypeOf(obj);
  const variant = getSchemaVariant(prototype);
  if (variant !== undefined) {
    writer.writeU8(variant);
    serializeStruct([...path, `<Variant[${variant}]>`], obj, writer);
  } else {
    invalidEnumField(path);
  }
}

function serializeStruct<T>(path: string[], obj: T, writer: BinaryWriter) {
  const prototype = Object.getPrototypeOf(obj);
  const fields = getSchemaFields(prototype);
  for (const field of fields) {
    serializeField(path, obj, field, writer);
  }
}

function deserializeEnum(path: string[], enumClass: Function, reader: BinaryReader): any {
  const prototype = enumClass.prototype;
  const enumVariants = getSchemaEnumVariants(prototype);
  if (enumVariants instanceof Array) {
    const i = reader.readU8();
    const variant = enumVariants[i];
    let obj = deserializeStruct([...path, `<Variant[${i}]>`], variant, reader);
    Object.setPrototypeOf(obj, variant.prototype);
    return obj;
  } else {
    invalidEnumField(path);
  }
}

function deserializeStruct<T>(path: string[], ctor: Ctor<T>, reader: BinaryReader): T {
  if (ctor === undefined) invalidCtor(path);
  const prototype = ctor.prototype;
  const fields = getSchemaFields(prototype);
  let obj = {};
  try {
    for (const field of fields) {
      obj = deserializeField(path, obj, field, reader);
    }
  } catch (e) {
    if (e instanceof DeserializeError) {
      if (e.obj === undefined) {
        e.obj = obj;
      }
    }
    throw e;
  }
  return new ctor(obj);
}

export function field(fieldType: FieldType) {
  return function (target: any, key: PropertyKey) {
    if (target?.constructor?.prototype === undefined)
      throw new Error(`Invalid field argument for key: ${key.toString()}`);
    addSchemaField(target.constructor.prototype, key, fieldType);
  };
}

export function variant(variant: number) {
  return function <T>(target: Ctor<T>) {
    addSchemaVariant(target, variant);
  };
}

export function extend<T>(options: ExtendOptions<T>): ExtendFieldType<T> {
  return { kind: 'extend', value: options };
}

export function array(elementType: FieldType): ArrayFieldType {
  return { kind: 'array', value: elementType };
}

export function map(keyType: FieldType, valueType: FieldType): MapFieldType {
  return { kind: 'map', value: [keyType, valueType] };
}

export function option(innerType: FieldType): OptionFieldType {
  return { kind: 'option', value: innerType };
}

export function struct<T>(ctor: Ctor<T>): StructFieldType<T> {
  return { kind: 'struct', value: ctor };
}

export function enums(enumClass: Function): EnumFieldType {
  return { kind: 'enum', value: enumClass };
}

export function serialize(obj: any): Uint8Array {
  const writer = new BinaryWriter();
  try {
    if (isVariantObject(obj)) {
      serializeEnum([], obj, writer);
    } else {
      serializeStruct([], obj, writer);
    }
  } catch (e) {
    if (e instanceof SerializeError) {
      console.error(
        'Serialize failed, path:',
        e.path,
        ', fieldType:',
        e.fieldType,
        ', value:',
        e.value,
        ', cause:',
        e.cause
      );
    }
    throw e;
  }
  return writer.toArray();
}

export function deserialize<T>(enumClass: EnumClass<T>, data: Uint8Array): T;
export function deserialize<T>(ctor: Ctor<T>, data: Uint8Array): T;
export function deserialize<T>(classType: Ctor<T> | EnumClass<T>, data: Uint8Array): T {
  const reader = new BinaryReader(data);
  try {
    if (isEnumClass(classType)) {
      return deserializeEnum([], classType, reader);
    } else {
      return deserializeStruct([], classType, reader);
    }
  } catch (e) {
    if (e instanceof DeserializeError) {
      console.error(
        'Deserialize failed, path:',
        e.path,
        ', current object:',
        e.obj,
        ', cause:',
        e.cause,
        ', data:',
        data
      );
    }
    throw e;
  }
}

export * from './types';
