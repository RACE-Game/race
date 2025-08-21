---
description: >-
  This section is for developers who want to contribute to the RACE Protocol JS
  SDK itself, whether by fixing bugs, adding new features, or improving the
  documentation.
---

# 🧑‍💻 For Contributors

## **Project Structure**

The RACE JS SDK is a TypeScript monorepo managed with **npm workspaces**. This structure allows us to maintain separate, independently versioned packages while sharing common configurations and dependencies. All packages are located in the `packages/` directory.

* **`packages/borsh`** (`@race-foundation/borsh`)\
  A custom implementation of the Borsh serialization format with decorator support. This is a low-level utility used across the SDK to ensure consistent data serialization for on-chain programs and network communication.
* **`packages/sdk-core`** (`@race-foundation/sdk-core`)\
  The central package containing the core logic, interfaces, and type definitions that are shared across all blockchain implementations. It is platform-agnostic and defines the essential abstractions like `ITransport`, `AppClient`, and the various account models.
* **`packages/sdk-solana`** (`@race-foundation/sdk-solana`)\
  The implementation of the `ITransport` interface and other core concepts specifically for the **Solana** blockchain. It handles Solana-specific instructions, account structures, and transaction serialization.
* **`packages/sdk-sui`** (`@race-foundation/sdk-sui`)\
  The implementation of the `ITransport` interface and other core concepts specifically for the **Sui** blockchain. It handles Sui-specific transactions, objects, and data parsing.
* **`packages/sdk-facade`** (`@race-foundation/sdk-facade`)\
  A mock implementation of the transport layer, designed for local development, testing, and tutorials. It simulates blockchain interactions without needing a live network connection, making it easy to get started and run examples.
* **`packages/config`**\
  A shared internal package that contains common configuration files for TypeScript (`tsconfig.json`), Jest (`jest.config.js`), and Prettier (`.prettierrc`), ensuring consistency across the entire monorepo.

## **Setting Up the Development Environment**

The repository uses **Nix flakes** to provide a reproducible development environment with all the necessary dependencies.

### **Prerequisites:**

1. **Git**: To clone the repository.
2. **Nix**: You must have the Nix package manager installed. Follow the [official installation guide](https://nixos.org/download/).
3. **Direnv** (Recommended): A tool that automatically loads and unloads environment variables as you change directories. This works seamlessly with Nix flakes. [Install direnv](https://direnv.net/docs/installation.html) and hook it into your shell.

### **Setup Steps:**

1.  **Clone the Repository:**

    ```bash
    git clone https://github.com/RACE-Game/race-sdk.git
    cd race-sdk
    ```
2.  **Activate the Nix Shell:**\
    If you have `direnv` installed and hooked into your shell, simply allow it to run when you enter the directory:

    ```bash
    direnv allow
    ```

    This command reads the `.envrc` file, which contains `use flake`. It will build and activate the Nix development shell, installing all the necessary tools like Node.js, `just`, and TypeScript into your current shell session.

    If you are not using direnv, you can activate the shell manually with:

    ```bash
    nix develop
    ```
3.  **Install Dependencies:**\
    Once your Nix shell is active, install all the npm dependencies for the workspaces using the Just task runner (which is provided by the Nix environment):

    ```bash
    just deps
    ```

    This command is a shortcut for `npm install --workspaces`, which will install all dependencies for every package in the monorepo.

You are now ready to start developing!

## **Building, Linting, and Formatting**

We use **Just** as our primary task runner for common development commands, along with standard npm scripts.

### **Building the Code**

The SDK is built into multiple formats (ESM, CJS, and TypeScript definitions) located in the `lib/` directory of each package.

*   **Build all packages:**

    ```bash
    just build-all
    ```
*   **Build a single package:**

    ```bash
    # Usage: just build <package-name>
    just build sdk-core
    ```

    Alternatively, you can use the npm workspace command:

    ```bash
    npm run build --workspace=@race-foundation/sdk-core
    ```

### **Type Checking**

You can run the TypeScript compiler to check for type errors without generating build artifacts. This is useful for quick verification.

```bash
# Usage: just check <package-name>
just check sdk-solana
```

### **Linting and Formatting**

We use ESLint for linting and Prettier for code formatting to maintain a consistent style.

*   **Check for linting errors in a specific package:**

    ```bash
    npm run lint --workspace=@race-foundation/sdk-core
    ```
*   **Automatically format all files:**

    ```bash
    npm run format --workspaces
    ```

## **Running Tests**

The project uses **Jest** as its testing framework. Test files are located in the `tests/` directory within each package and end with the `.spec.ts` extension.

*   **Run all tests across all packages:**

    ```bash
    npm test
    ```
*   **Run tests for a specific package:**

    ```bash
    # Example for sdk-core
    npm test --workspace=@race-foundation/sdk-core
    ```
*   **Run tests in watch mode:**\
    To automatically re-run tests when files change, append the `--watch` flag to the test command.

    ```bash
    # Watch all packages
    npm test -- --watch

    # Watch a single package
    npm test --workspace=@race-foundation/sdk-core -- --watch
    ```

## **Publishing Packages (For Maintainers)**

Publishing is handled by `npm` from within each package's directory, but the `Justfile` provides convenient shortcuts.

*   **Publish a single package to NPM:**

    ```bash
    # Usage: just publish-npmjs <package-name>
    just publish-npmjs borsh
    ```
*   **Publish all packages to NPM:**

    ```bash
    just publish-npmjs-all
    ```

    This command will build and publish each package in the correct dependency order. Ensure you have the necessary permissions on npmjs.org before running this.
