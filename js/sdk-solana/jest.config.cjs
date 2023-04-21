module.exports = {
  preset: "ts-jest",
  extensionsToTreatAsEsm: [".ts"],
  moduleNameMapper: {
    '^(\\.{1,2}/.*)\\.js$': '$1',
  },
  roots: ["<rootDir>/tests"],
  transform: {
    '^.+\\.tsx?$': ['ts-jest', {useESM: true,},],
  },
};
