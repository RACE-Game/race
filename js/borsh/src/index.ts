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
import { invalidByteArrayLength, extendedWriterNotFound, extendedReaderNotFound, invalidEnumField } from './errors';

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
          serializeValue([...path, `<Array[${i}]>`], value[i], v, writer);
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
    console.error('Borsh serialize failed, path:', path, 'error:', err);
    throw err;
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
          let v = deserializeValue([...path, `<Array[${i}]>`], value, reader);
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
    console.error('Borsh serialize failed, path:', path, 'error:', err);
    throw err;
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
    return deserializeStruct([...path, `<Variant[${i}]>`], variant, reader);
  } else {
    invalidEnumField(path);
  }
}

function deserializeStruct<T>(path: string[], ctor: Ctor<T>, reader: BinaryReader): T {
  const prototype = ctor.prototype;
  const fields = getSchemaFields(prototype);
  let obj = {};
  for (const field of fields) {
    obj = deserializeField([...path, field[0].toString()], obj, field, reader);
  }
  return new ctor(obj);
}

export function field(fieldType: FieldType) {
  return function (target: any, key: PropertyKey) {
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
  if (isVariantObject(obj)) {
    serializeEnum([], obj, writer);
  } else {
    serializeStruct([], obj, writer);
  }
  return writer.toArray();
}

export function deserialize<T>(enumClass: EnumClass<T>, data: Uint8Array): T;
export function deserialize<T>(ctor: Ctor<T>, data: Uint8Array): T;
export function deserialize<T>(classType: Ctor<T> | EnumClass<T>, data: Uint8Array): T {
  const reader = new BinaryReader(data);
  if (isEnumClass(classType)) {
    return deserializeEnum([], classType, reader);
  } else {
    return deserializeStruct([], classType, reader);
  }
}

export * from './types';
