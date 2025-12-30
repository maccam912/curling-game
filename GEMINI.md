# Curling Game

## Project Overview
This project is a 3D curling game built using the **Rust** programming language and the **Bevy** game engine (version 0.17). It simulates the physics of curling stones on ice using `bevy_rapier2d` and implements standard curling rules like the Free Guard Zone (FGZ) and Hog Line violations.

## Building and Running

### Prerequisites
*   Rust toolchain (stable)
*   System dependencies for Linux (see below)

### Commands
*   **Run the Game (Debug - Recommended):**
    ```bash
    cargo run
    ```
    > [!NOTE]
    > Debug builds are significantly faster to compile. The `Cargo.toml` is configured to optimize dependencies even in debug mode, so performance remains acceptable for development.

*   **Run the Game (Release):**
    ```bash
    cargo run --release
    ```
    > [!WARNING]
    > Release builds take a very long time due to LTO and full optimization. Use only for final distribution.

*   **Run Tests:**
    ```bash
    cargo test
    ```
*   **Format Code:**
    ```bash
    cargo fmt
    ```

### System Dependencies (Linux)

Bevy requires system libraries for windowing and audio. Install before building or running tests:

```bash
# Ubuntu/Debian
sudo apt-get install libwayland-dev libxkbcommon-dev libasound2-dev libudev-dev

# Fedora
sudo dnf install wayland-devel libxkbcommon-devel alsa-lib-devel systemd-devel
```

> [!NOTE]
> These are only needed for building/testing on Linux. WASM builds (for web deployment) don't require these.

## Project Structure

The project is organized into modular files for maintainability:

```
src/
├── main.rs              # Minimal entry point (app setup)
├── lib.rs               # Library root with CurlingPlugin
├── constants.rs         # All game/physics constants
├── components.rs        # Bevy ECS components and enums
├── resources.rs         # Bevy ECS resources (GameState, etc.)
├── helpers.rs           # Utility functions (spawn, predicates)
└── systems/
    ├── mod.rs           # System module re-exports
    ├── setup.rs         # Scene and UI initialization
    ├── input.rs         # Keyboard, mouse, touch handling
    ├── physics.rs       # Ice friction and collision
    ├── camera.rs        # Camera control and transitions
    ├── game_logic.rs    # Shot resolution, rule enforcement
    └── ui.rs            # UI updates and display
```

## Development Conventions

*   **Architecture:** The project follows the Bevy ECS (Entity Component System) pattern.
    *   **Components** (`components.rs`): `Stone`, `ThrowingStone`, `Broom`, `MainCamera`, UI markers, and enums (`Team` (One/Two with customizable colors), `Phase`, `CameraMode`, `ShotType`, `CurlDirection`).
    *   **Resources** (`resources.rs`): `GameState` (manages turns, shot parameters, and game phases with `team1_score`/`team2_score`), `CameraState`, `TouchState`, `StoneAssets`.
    *   **Systems** (`systems/`): Logic is organized by functionality:
        - `setup.rs`: Scene and UI initialization
        - `input.rs`: Input handling (`handle_calling_input`, `handle_aiming_input`, `handle_broom_drag`, `handle_touch_input`)
        - `physics.rs`: Physics simulation (`ice_friction_system`, `track_throwing_stone`, `detect_stone_collision`)
        - `camera.rs`: Camera transitions (`camera_control_system`)
        - `game_logic.rs`: Rule enforcement (`resolve_shot`, `check_out_of_bounds`, `detect_shot_end`)
        - `ui.rs`: UI updates (`update_ui`, `update_window_title`, `update_broom_visual`)
        - `ai.rs`: AI opponent logic (`ai_turn_system`, `setup_ai_game`, shot decision logic)
*   **Logging:** Structured logging using the `tracing` crate. Set log level with `RUST_LOG` environment variable (e.g., `RUST_LOG=curling_game=debug`).
*   **Testing:** Unit tests are in `src/lib.rs` in the `tests` module, covering team alternation, coordinate calculations, rule predicates, physics constants, and ice friction physics (71 tests total).

## Game Logic & Rules

The game operates in phases managed by the `GameState` resource:
1.  **Calling Shot:** The skip calls the desired angle and weight (drag broom to aim).
2.  **Aiming:** The player fine-tunes the aim and executes the throw.
3.  **Stone Moving:** The game waits for all stones to come to rest.
4.  **Resolve:** Rules are applied:
    *   **Near Hog Line:** Stones that don't cross the near hog line during delivery are removed.
    *   **Far Hog Line:** Stones that don't reach the far hog line are removed (unless they hit another stone).
    *   **Free Guard Zone (FGZ):** If a guard stone (in the FGZ) is removed by the opponent within the first 5 shots, it is replaced, and the shooter is removed.
    *   **Out of Bounds:** Stones leaving the sheet boundaries are despawned immediately.

## Controls

*   **Mouse Drag:** Drag the broom to set shot angle and weight.
*   **Arrow Keys / WASD:** Adjust angle (aim) and weight (force).
*   **Enter:** Confirm the called shot (transition from Calling to Aiming).
*   **Space:** Throw the stone (transition from Aiming to Stone Moving).
*   **R:** Reset aim to the originally called shot parameters.
*   **C:** Toggle camera view (SkipView / Overhead) during shot calling.
*   **UI Buttons:** VIEW (camera toggle), IN/OUT (curl direction), Confirm/Throw.
