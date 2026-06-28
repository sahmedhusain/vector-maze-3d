# Getting Started Guide 🚀

This guide will walk you through compiling, running, and playing **Maze Wars 3D**. 

---

## 📋 Table of Contents
1. [First Steps & Prerequisites](#-first-steps--prerequisites)
2. [Compiling the Code](#-compiling-the-code)
3. [Setting up the Server](#-setting-up-the-server)
4. [Launching the Client](#-launching-the-client)
5. [Using the Connection Launcher](#-using-the-connection-launcher)
6. [How the Lobby Works](#-how-the-lobby-works)
7. [In-Game Controls](#-in-game-controls)
9. [Setting up LAN Matches](#-setting-up-lan-matches)

---

## 🛠 First Steps & Prerequisites

To build and run this project, you will need the Rust toolchain on your computer. If you don't have it yet, you can get it by running:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Once installed, restart your terminal and verify it works:
```bash
rustc --version
cargo --version
```

---

## 🏗 Compiling the Code

You can build the project by running:

```bash
# Compiles both server and client binaries
cargo build --release

# Quickly checks if the code compiles without producing the output binary
cargo check
```

---

## ⚙️ Setting up the Server

The server keeps track of player coordinates, spawns, scores, lasers, and AI bots. It accepts a few flags to customize settings:

### Commands
```bash
cargo run --release --bin server -- [options]
```

### Options
| Option | What it does | Default |
| :--- | :--- | :--- |
| `--port, -p <port>` | Port to bind and listen on (UDP) | `10500` |
| `--bots, -b <true/false>` | Enable or disable AI bots | `true` |
| `--level, -l <level_idx>` | Initial map index (0 to 3) | `0` |
| `--help, -h` | Shows usage instructions | N/A |

### Example
To host a match on port `12345`, on Level 2 (Labyrinth), and with bots disabled:
```bash
cargo run --release --bin server -- --port 12345 --level 2 --bots false
```

---

## 🎮 Launching the Client

The client connects to a server, shows the wireframe 3D view, displays the scoreboard, and lets you play.

### Connecting via GUI Launcher (Recommended)
Simply start the client binary without any flags. This opens the main menu launcher where you can manage your favorite servers:
```bash
cargo run --release --bin client
```

### Direct Connection (CLI Bypass)
If you want to skip the main menu and connect directly, pass the IP address and your username as arguments:
```bash
# Using named arguments
cargo run --release --bin client -- --ip 127.0.0.1:10500 --name Sayed

# Shorthand positional arguments
cargo run --release --bin client -- 127.0.0.1:10500 Sayed
```

---

## 🖥 Using the Connection Launcher

If you launch the client without arguments, you'll see a responsive, modern connection launcher.

- **Form Fields**: Fill in the server IP (e.g. `127.0.0.1:10500`), your username, and an optional nickname/alias (like "Main Server").
- **Tab Key Navigation**: Press `Tab` to cycle between input fields, or click on them directly.
- **Holding Backspace**: If you want to erase a field quickly, hold down the Backspace key.
- **Connection History**: The launcher saves your history to `hosts_history.txt`. Next time you play, click on any saved row in the history list to fill in the inputs instantly.
- **Delete old entries**: Click the red `[x]` next to a saved server to remove it from your history list.

---

## 👥 How the Lobby Works

When you connect, you enter the lobby. Here are the rules of the lobby:
- **Who is the Host?** The first player to connect to the server becomes the Host. Guests will see `"Waiting for host to start..."` on their screens.
- **What can the Host do?** Only the Host can change the level, toggle bots, or start the match.
- **Lobby Keys for the Host**:
  - `1`, `2`, `3` — Load pre-designed static maps (Level 0, 1, 2).
  - `4` — Generate a random level map using the DFS maze algorithm.
  - `B` — Toggle AI bots (spawns bots to reach the 4-player cap, or clears them).
  - `G` — Start the match.

---

## 🕹 In-Game Controls

Once the match starts:

- **Movement**:
  - `W` / `Up Arrow` — Move forward one grid unit.
  - `S` / `Down Arrow` — Move backward one grid unit.
  - `A` / `Left Arrow` — Turn left 90 degrees.
  - `D` / `Right Arrow` — Turn right 90 degrees.
- **Combat**:
  - `Space` or `Mouse Left Click` — Shoot a laser forward down your sight line.
- **Leave Match**:
  - `ESC` — Disconnect from the match and return to the main connection launcher.

---

---

## 🌐 Setting up LAN Matches

To play with friends on the same local Wi-Fi or Ethernet network:

1. **Find your Local IP Address**:
   - On macOS: Run `ipconfig getifaddr en0` in the terminal.
   - On Windows: Run `ipconfig` in the command prompt.
2. **Start the Server**:
   Host the server on your machine:
   ```bash
   cargo run --release --bin server -- --port 10500
   ```
3. **Connect the Clients**:
   Give your local IP (e.g. `192.168.1.53`) to your friends. They can launch their clients, enter your IP and port (`192.168.1.53:10500`), type their nickname, and click Connect!

### Running a Local Test Sandbox
If you want to test multiplayer mechanics on a single computer, open three terminal windows:

```bash
# Terminal 1: Start the server
cargo run --bin server

# Terminal 2: Connect first client (Host)
cargo run --bin client -- 127.0.0.1:10500 HostPlayer

# Terminal 3: Connect second client (Guest)
cargo run --bin client -- 127.0.0.1:10500 FriendPlayer
```
Press `B` in the Host terminal to spawn bots, and `G` to start the game!
