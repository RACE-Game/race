module.exports = {
  preset: 'ts-jest',
  testEnvironment: 'node',
  extensionsToTreatAsEsm: [".ts"],
  testMatch: ['<rootDir>/tests/**/*.spec.ts']
};
