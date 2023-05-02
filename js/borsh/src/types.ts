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

export type ArrayFieldType = ['array', FieldType];

export type MapFieldType = ['map', FieldType];

export type OptionFieldType = ['option', FieldType];

export type StructFieldType = ['struct', Function];

export type FieldType = PrimitiveFieldType
  | ArrayFieldType
  | MapFieldType
  | OptionFieldType
  | StructFieldType;

export type Field = [FieldKey, FieldType];
