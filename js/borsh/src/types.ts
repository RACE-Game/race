export type EnumClass<T> = Function & { prototype: T };
export type Ctor<T> = { new (_: any): T };

export interface IExtendReader<T> {
  read(buf: Uint8Array, offset: number): T;
}

export interface IExtendWriter<T> {
  write(value: T, buf: Uint8Array, offset: number): void;
}

export type ExtendOptions<T> = {
  size: number;
  reader?: IExtendReader<T>;
  writer?: IExtendWriter<T>;
};

export type HasExtendedWriter<T> = Required<Pick<ExtendOptions<T>, 'writer' | 'size'>>;

export type HasExtendedReader<T> = Required<Pick<ExtendOptions<T>, 'reader' | 'size'>>;

export type PrimitiveFieldType =
  | 'u8'
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
  | 'usize'
  | 'bool'
  | 'u8-array';

export type FieldKey = PropertyKey;

export type ByteArrayFieldType = number;

export type EnumFieldType = { kind: 'enum'; value: Function };

export type ArrayFieldType = { kind: 'array'; value: FieldType };

export type MapFieldType = { kind: 'map'; value: [FieldType, FieldType] };

export type OptionFieldType = { kind: 'option'; value: FieldType };

export type StructFieldType<T> = { kind: 'struct'; value: Ctor<T> };

export type ExtendFieldType<T> = { kind: 'extend'; value: ExtendOptions<T> };

export type FieldType =
  | PrimitiveFieldType
  | ByteArrayFieldType
  | ArrayFieldType
  | MapFieldType
  | OptionFieldType
  | EnumFieldType
  | StructFieldType<any>
  | ExtendFieldType<any>;

export type Field = [FieldKey, FieldType];

export function isPrimitiveType(fieldType: FieldType): fieldType is PrimitiveFieldType {
  return typeof fieldType === 'string';
}

export function hasExtendWriter<T>(options: ExtendOptions<T>): options is HasExtendedWriter<T> {
  return options.writer !== undefined;
}

export function hasExtendReader<T>(options: ExtendOptions<T>): options is HasExtendedReader<T> {
  return options.reader !== undefined;
}
