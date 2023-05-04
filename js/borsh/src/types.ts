export interface IExtendReader<T> {
  read(buf: Uint8Array, offset: number): T;
}

export interface IExtendWriter<T> {
  write(value: T, buf: Uint8Array, offset: number): void;
}

export type ExtendOptions<T> = {
  size: number,
  reader?: IExtendReader<T>,
  writer?: IExtendWriter<T>,
}

export type PrimitiveFieldType = 'u8'
  | 'u16'
  | 'u32'
  | 'u64'
  | 'u128'
  | 'i8'
  | 'i16'
  | 'i32'
  | 'i64'
  | 'i128'
  | 'string'
  | 'f32'
  | 'f64'
  | 'bool';

export type FieldKey = PropertyKey;

export type ByteArrayFieldType = number;

export type VecFieldType = { kind: 'vec', value: FieldType };

export type MapFieldType = { kind: 'map', value: FieldType };

export type OptionFieldType = { kind: 'option', value: FieldType };

export type StructFieldType = { kind: 'struct', value: Function };

export type ExtendFieldType<T> = { kind: 'extend', value: ExtendOptions<T> };

export type FieldType = PrimitiveFieldType
  | ByteArrayFieldType
  | VecFieldType
  | MapFieldType
  | OptionFieldType
  | StructFieldType
  | ExtendFieldType<any>;

export type Field = [FieldKey, FieldType]
