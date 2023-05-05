import { VecFieldType, Field, FieldKey, FieldType, OptionFieldType, StructFieldType, ExtendOptions, ExtendFieldType, Ctor, isPrimitiveType, hasExtendReader, hasExtendWriter, EnumFieldType } from './types';
import { BinaryWriter } from "./writer";
import { BinaryReader } from "./reader";


function addSchemaField(prototype: any, key: FieldKey, fieldType: FieldType) {
  let fields: Field[] = prototype.__schema_fields || [];
  fields.push([key, fieldType]);
  prototype.__schema_fields = fields;
}

function getSchemaFields(prototype: any): Field[] {
  return prototype.__schema_fields
}

function addSchemaVariant(prototype: any, variant: number) {
  prototype.__schema_variant = variant;
  const superClass = Object.getPrototypeOf(prototype).constructor;
  let enumVariants = superClass.prototype.__schema_enum_variants || [];
  enumVariants.push(prototype);
  superClass.prototype.__schema_enum_variants = enumVariants;
}

function getSchemaVariant(prototype: any): number | undefined {
  return prototype.__schema_variant
}

function getSchemaEnumVarients(prototype: any): any[] | undefined {
  return prototype.__schema_enum_variants;
}

function serializeValue(path: string[], value: any, fieldType: FieldType, writer: BinaryWriter) {
  if (isPrimitiveType(fieldType)) {
    if (fieldType === 'u8') {
      writer.writeU8(value);
    }
    else if (fieldType === 'u16') {
      writer.writeU16(value);
    }
    else if (fieldType === 'u32') {
      writer.writeU32(value);
    }
    else if (fieldType === 'u64') {
      writer.writeU64(value);
    }
    else if (fieldType === 'bool') {
      writer.writeBool(value);
    }
    else if (fieldType === 'string') {
      writer.writeString(value);
    }
  } else if (typeof fieldType === 'number') {
    if (value.length !== fieldType) {
      invalidByteArrayLength(path, fieldType, value.length);
    }
    writer.writeByteArray(value);
  }
  else {
    const { kind, value: v } = fieldType;
    if (kind === 'option') {
      if (value === undefined || value === null) {
        writer.writeU8(0);
      } else {
        writer.writeU8(1);
        serializeValue([...path, '<OptionValue>'], value, v, writer);
      }
    }
    else if (kind === 'vec') {
      writer.writeU32(value.length);
      for (let i = 0; i < value.length; i++) {
        serializeValue([...path, `<Vec[${i}]>`], value[i], v, writer);
      }
    }
    else if (kind === 'struct') {
      serializeStruct(path, value, writer);
    }
    else if (kind === 'enum') {
      serializeEnum(path, value, writer);
    }
    else if (kind === 'extend') {
      if (hasExtendWriter(v)) {
        writer.writeExtended(value, v);
      } else {
        extendedWriterNotFound(path);
      }
    }
  }
}

function deserializeValue(path: string[], fieldType: FieldType, reader: BinaryReader): any {
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
    } else if (fieldType === 'string') {
      return reader.readString();
    }
  }
  else if (typeof fieldType === 'number') {
    return reader.readByteArray(fieldType);
  }
  else {
    const { kind, value } = fieldType;
    if (kind === 'vec') {
      let vec = [];
      const length = reader.readU32();
      for (let i = 0; i < length; i++) {
        let v = deserializeValue([...path, `<Vec[${i}]>`], value, reader);
        vec.push(v);
      }
      if (value === 'u8') {
        return Uint8Array.from(vec);
      } else {
        return vec;
      }
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
    } else if (kind === 'extend') {
      if (hasExtendReader(value)) {
        return reader.readExtended(value);
      } else {
        extendedReaderNotFound(path);
      }
    }
  }
}

function serializeField(path: string[], obj: any, field: Field, writer: BinaryWriter) {
  const [key, fieldType] = field;
  const k = key.toString();
  const value = obj[k];
  serializeValue([...path, k], value, fieldType, writer)
}

function deserializeField(path: string[], obj: Record<string, any>, field: Field, reader: BinaryReader): Object {
  const [key, fieldType] = field;
  const k = key.toString();
  const value = deserializeValue([...path, k], fieldType, reader);
  obj[k] = value;
  return obj;
}

function serializeEnum<T>(path: string[], obj: T, writer: BinaryWriter) {
  const prototype = Object.getPrototypeOf(obj)
  const variant = getSchemaVariant(prototype);
  if (variant !== undefined) {
    writer.writeU8(variant);
    serializeStruct([...path, `<Variant[${variant}]>`], obj, writer);
  } else {
    invalidEnumField(path);
  }
}

function serializeStruct<T>(path: string[], obj: T, writer: BinaryWriter) {
  const prototype = Object.getPrototypeOf(obj)
  const fields = getSchemaFields(prototype);
  for (const field of fields) {
    serializeField(path, obj, field, writer);
  }
}

function deserializeEnum(path: string[], enumClass: Function, reader: BinaryReader): any {
  const prototype = enumClass.prototype;
  const enumVariants = getSchemaEnumVarients(prototype);
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
    obj = deserializeField(path, obj, field, reader);
  }
  return new ctor(obj);
}

export function field(fieldType: FieldType) {
  return function(target: any, key: PropertyKey) {
    addSchemaField(target.constructor.prototype, key, fieldType);
  }
}

export function variant(variant: number) {
  return function(target: Function) {
    addSchemaVariant(target.prototype, variant);
  }
}

export function extend<T>(options: ExtendOptions<T>): ExtendFieldType<T> {
  return { kind: 'extend', value: options };
}

export function vec(elementType: FieldType): VecFieldType {
  return { kind: 'vec', value: elementType };
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
  serializeStruct([], obj, writer);
  return writer.toArray();
}

export function deserialize<T>(ctor: Ctor<T>, data: Uint8Array): T {
  const reader = new BinaryReader(data);
  return deserializeStruct([], ctor, reader);
}
