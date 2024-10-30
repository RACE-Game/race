export function invalidCtor(path: string[]) {
  throw new Error(`Borsh: Cannot deserialize, missing type annotation at: ${path.join(',')}`);
}

export function invalidByteArrayLength(path: string[], expected: number, actual: number) {
  throw new Error(`Borsh: Invalid byte array length at: ${path.join(',')}, expected: ${expected}, actual: ${actual}`);
}

export function unexpectedFieldSchema(path: string[], expected: string, actual: string) {
  throw new Error(`Borsh: Found an unexpected schema at: ${path.join(',')}, expected: ${expected}, actual: ${actual}`);
}

export function extendedWriterNotFound(path: string[]) {
  throw new Error(`Borsh: Extended writer not found at: ${path.join(',')}`);
}

export function extendedReaderNotFound(path: string[]) {
  throw new Error(`Borsh: Extended reader not found at: ${path.join(',')}`);
}

export function noSuperClassForVariant(cls: Function) {
  throw new Error(`Borsh: No super class available for class ${cls} which is decorated as variant`);
}

export function invalidEnumField(path: string[]) {
  throw new Error(`Borsh: Invalid enum field type at: ${path.join(',')}`);
}
