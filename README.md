# proxy-lib

Man-in-the-middle proxy library for kkowa proxy collector. Currently being used for internal purpose only.

## 🧰 Tech Stack

- **Language** Rust Nightly

- **Framework** [hyper](https://hyper.rs/)

- **CI·CD** GitHub Actions

## ⚙️ Getting Started

This section describes how to set your local development environments up.

### **(A)** Developing Inside Container

Requirement:

- [Docker](https://www.docker.com/)

  To configure other dependent services like database, we use Docker (mainly [Docker Compose](https://docs.docker.com/compose/)).

- [Visual Studio Code](https://code.visualstudio.com/)

  VS Code Development Container provides rich features such as git and GnuPG configuration forwarding. But they sometimes require you to install some tools based on your device. Please check [this](https://code.visualstudio.com/docs/remote/containers#_sharing-git-credentials-with-your-container).

As container itself configured to include all required tools, there's no extra tools required.

1. Install VS Code extension [Dev Containers](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers).

1. Then, clone this repository and open in VS Code, select **Remote-Containers: Reopen in Container...** at command palette (<kbd>ctrl</kbd> + <kbd>shift</kbd> + <kbd>P</kbd> or <kbd>cmd</kbd> + <kbd>shift</kbd> + <kbd>P</kbd>).

1. Done.

### **(B)** Developing Locally

Requirement:

- [pre-commit](https://pre-commit.com/)

Follow next for local development setup:

1. Run `make install`. Some additional dev-tools will be downloaded to **.bin** directory.

1. Run `make init`

1. Done. all other configurations such as managing environment variables, setting up databases are on your own. You can use existing docker compose configuration to manage them (but it would require some configuration changes).

### ⌨️ Basic Commands

Commands repeatedly used are defined in [Makefile](./Makefile). Just type `make` without arguments will show you possible commands.
