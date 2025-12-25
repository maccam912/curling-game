# Curling Game

## Project Overview
A 3D curling game built in **Rust** using the **Bevy** game engine (v0.17.3). The game simulates curling physics via `bevy_rapier2d` and implements standard curling rules including the Free Guard Zone (FGZ) and Hog Line violations.

## Quick Start

```bash
# Run the game (Debug - Recommended)
cargo run

# Run the game (Release - Slow)
cargo run --release

# Run tests
cargo test

# Format code
cargo fmt
```

> [!TIP]
> **Build Performance:** Release builds (`--release`) take a long time. Use debug builds for development. Dependencies are optimized in `Cargo.toml` (`opt-level = 3` for deps) to ensure smooth performance even in debug mode.

## Architecture

This project follows the **Bevy ECS** (Entity Component System) pattern. All code is in `src/main.rs`.

### Components
| Component | Purpose |
|-----------|---------|
| `Stone` | Stores team info (`Red` or `Blue`) |
| `ThrowingStone` | Tracks delivery max Y for hog line checks |

### Resources
| Resource | Purpose |
|----------|---------|
| `GameState` | Manages turns, shot parameters, and game phases |
| `StoneAssets` | Caches mesh and material handles for stones |

### Key Systems
- `handle_calling_input` - Skip calls the shot angle and weight
- `handle_aiming_input` - Player fine-tunes aim and throws
- `track_throwing_stone` - Tracks stone position for hog line validation
- `check_out_of_bounds` - Despawns stones leaving the sheet
- `detect_shot_end` - Waits for stones to stop moving
- `resolve_shot` - Applies rules (hog line, FGZ) after each shot

### Game Phases
1. **CallingShot** - Skip calls the desired angle and weight
2. **Aiming** - Player fine-tunes aim and executes throw
3. **StoneMoving** - Game waits for all stones to rest
4. **Resolve** - Rules are applied
5. **Ended** - End is complete

## Key Constants

```rust
const SHEET_LENGTH: f32 = 45.72;     // 150 ft
const SHEET_WIDTH: f32 = 4.75;       // ~15 ft 7 in
const TEE_FROM_CENTER: f32 = 17.375; // 57 ft
const STONE_RADIUS: f32 = 0.145;     // Standard curling stone
const TOTAL_SHOTS: u8 = 16;          // 8 per team
```

## Game Rules

### Hog Line Violation
Stones that don't cross the near hog line during delivery are removed.

### Free Guard Zone (FGZ)
Active for the first 5 shots. If a guard stone (in the FGZ but outside the house) is knocked out by the opponent, the guard is restored to its original position and the shooter is removed.

### Out of Bounds
Stones leaving the sheet boundaries are immediately despawned.

## Controls

| Key | Action |
|-----|--------|
| Arrow Keys / WASD | Adjust angle (horizontal) and weight (vertical) |
| Enter | Confirm called shot (Calling → Aiming) |
| Space | Throw the stone (Aiming → Stone Moving) |
| R | Reset aim to originally called shot parameters |

## Dependencies

- `bevy = "0.17.3"` - Game engine
- `bevy_rapier2d = "0.32.0"` - 2D physics (used for top-down curling physics)
- `bevy_renet` (optional) - Multiplayer support (feature: `multiplayer`)

## Testing

Unit tests are in `src/main.rs` in the `tests` module. They cover:
- Team alternation (`team_alternates_by_shot_index`)
- Coordinate symmetry (`hog_lines_are_symmetric`)
- Hog line validation (`hog_line_reached_when_crossing_delivery_hog_line`)
- Free guard zone detection (`free_guard_zone_detection`)
- Boundary detection (`out_of_bounds_checks_edges`)

## File Structure

```
curling-game/
├── .cargo/config.toml   # Cargo configuration
├── Cargo.toml           # Dependencies and features
├── src/main.rs          # All game code (~800 lines)
└── target/              # Build output (ignored)
```

## Development Notes

- The game uses 2D physics (`bevy_rapier2d`) with a 3D visual representation
- Coordinates: Y-axis is the length of the sheet, X-axis is the width
- Camera is positioned above, looking down at the sheet
- Physics has zero gravity (configured in `configure_rapier`)
- Ice friction is simulated via `Damping` component on stones
