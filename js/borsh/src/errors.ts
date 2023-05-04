

function invalidByteArrayLength(path: string[], expected: number, actual: number) {
  throw new Error(`Borsh: Invalid byte array length at: ${path.join(",")}, expected: ${expected}, actual: ${actual}`);
}

function unexpectedFieldSchema(path: string[], expected: string, actual: string) {
  throw new Error(`Borsh: Found an unexpected schema at: ${path.join(",")}, expected: ${expected}, actual: ${actual}`);
}

function extendedWriterNotFound(path: string[]) {
  throw new Error(`Borsh: Extended writer not found at: ${path.join(",")}`);
}

function noSuperClassForVariant(cls: Function) {
  throw new Error(`Borsh: No super class available for class ${cls} which is decorated as variant`)
}
