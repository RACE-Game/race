module.exports = {
  transform: {
    "\.ts$": ['ts-jest', {
      tsconfig: './tsconfig.test.json',
    }]
  },
  testEnvironment: 'node',
  testMatch: [
    '<rootDir>/tests/**/*.spec.ts',
  ],
}
