export const FORGEFILE_VERSION = "1" as const;

export const exampleForgefile = `version: "1"
name: example
jobs:
  test:
    name: Run Tests
  steps:
    - name: Echo
      run: echo hello
`;