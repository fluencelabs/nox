module.exports = {
  globals: {
    'ts-jest': {
      tsConfig: 'tsconfig.spec.json'
    }
  },
  'moduleFileExtensions': [
    'ts',
    'tsx',
    'js'
  ],
  transform: {
    '^.+\\.tsx?$': 'ts-jest'
  },
  testRegex: '(/__tests__/.*|(\\.|/)(test|spec))\\.tsx?$',
  testURL: 'http://localhost',
  testEnvironment: 'node',
  collectCoverageFrom: [
    'src/**/*.{js,ts,tsx}',
    '!src/**/index.ts',
    '!src/**/*.d.ts'
  ],
  coveragePathIgnorePatterns: [
    '<rootDir>/dist/',
    '<rootDir>/bundle/',
    '<rootDir>/node_modules/'
  ],
  coverageReporters: [
    'html',
    'text',
    'text-summary'
  ],
};
