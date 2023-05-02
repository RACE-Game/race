import { ArrayFieldType, Field, FieldKey, FieldType } from './types';
import { BinaryWriter } from "./writer";


export function addSchema(prototype: any, key: FieldKey, fieldType: FieldType) {
  let schema: Field[] = prototype.__schema || [];
  schema.push([key, fieldType]);
  prototype.__schema = schema;
}

export function getSchema(obj: Function): Field[] {
  return obj.prototype.__schema
}

export function field(fieldType: FieldType) {
  return function(target: any, key: PropertyKey) {
    addSchema(target.constructor.prototype, key, fieldType);
  }
}

export function array(elementType: FieldType): ArrayFieldType {
  return ['array', elementType];
}

export function serialize(obj: any): Uint8Array {
  const writer = new BinaryWriter();
  const fields = getSchema(Object.getPrototypeOf(obj));
  for (const field of fields) {
    const [key, fieldType] = field;
    const value = obj[key.toString()];
    if (fieldType === 'u8') {
      writer.writeU8(value);
    }
  }
  return writer.toArray();
}

export function deserialize<T>(ctor: { new(fields: any): T }, data: Uint8Array) {
  return new ctor({});
}
