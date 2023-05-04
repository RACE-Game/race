import { VecFieldType, Field, FieldKey, FieldType, OptionFieldType, PrimitiveFieldType, ByteArrayFieldType, StructFieldType, ExtendOptions, ExtendFieldType } from './types';
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

function isPrimitiveType(fieldType: FieldType): fieldType is PrimitiveFieldType {
  return typeof fieldType === 'string'
}

function hasExtendWriter<T>(options: ExtendOptions<T>): options is Required<Pick<ExtendOptions<T>, "writer" | "size">> {
  return options.writer !== undefined;
}

function hasExtendReader<T>(options: ExtendOptions<T>): options is Required<Pick<ExtendOptions<T>, "reader" | "size">> {
  return options.reader !== undefined;
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
        serializeValue([...path, `Vec(${i})`], value[i], v, writer);
      }
    }
    else if (kind === 'struct') {
      serializeStruct(path, value, writer);
    }
    else if (kind === 'extend') {
      if (hasExtendWriter(v)) {
        writer.writeExtended(value, v);
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

function deserializeField(path: string[], obj: Object, field: Field, reader: BinaryReader): Object {

  return obj;
}

function serializeStruct(path: string[], obj: any, writer: BinaryWriter) {
  const prototype = Object.getPrototypeOf(obj)
  const fields = getSchemaFields(prototype);
  const variant = getSchemaVariant(prototype);
  if (variant !== undefined) {
    writer.writeU8(variant);
  }
  for (const field of fields) {
    serializeField([], obj, field, writer);
  }
}

type Ctor<T> = { new(_: Object): T };

function deserializeStruct<T>(path: string[], ctor: Ctor<T>, reader: BinaryReader): T {
  let obj = {};
  const fields = getSchemaFields(ctor.constructor.prototype);
  for (const field of fields) {
    obj = deserializeField([], obj, field, reader);
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

export function struct(ctorType: Function): StructFieldType {
  return { kind: 'struct', value: ctorType };
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
