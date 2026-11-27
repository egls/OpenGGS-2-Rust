# OpenGGS-2-Rust

A pure-Rust rewrite of **OpenGGS 2.0 (OpenGGS Maker)** — a Free Software/Open Source remake of *The Great Giana Sisters*, with original music by **puch666** and original C++/SDL2 implementation by **Roman Högg** (<https://sourceforge.net/projects/openggs/>).

Built with **Bevy 0.15** and pure-Rust Impulse Tracker playback (**`xmrs` / `xmrsplayer`**), reading and writing the original OpenGGS 2.0 binary assets directly in `base/` (`*.lvl`, `Tilesheetinfo.tsi`, `Options.oi`, spritesheets, WAV SFX, and `.it` tracker music).

## Features
- **Full Stage & Dynamic Pack Support:** Automatically discovers all `*.lvl` packs in `base/stages/` (`classic.lvl`, `new.lvl`, `modules.lvl`, and custom packs), loads all 33 classic stages + 4 bonus substages with checkpoints, warp stones (`"WARPING!!!"` transition), substage entrances/exits, and trimmed camera bounds for short stages/substages.
- **Physics & Interactive World:** Fixed-timestep movement with coyote time, variable-height jump release, `ArrowDown` fast-fall, moving platforms, crumbling ground, animated tiles (coins, blocks, flames, lava), stage colour themes, Punk-only breakable crack animations, multi-hit 5-coin blocks, helmet coin blocks, falling solid drop stones, collectible coins (100 coins = +1 life), and power-up blocks.
- **Power-Ups & Combat:** Normal → Punk morphing → Fireball → Ricochet → Homing → Fire Shield → Extra Life, animated fireballs with impact explosion sprites.
- **Enemies:** All 15 classic enemy types with camera proximity activation, cliff-edge reversal, flying, leaping, Dragon box patrol, Spider horizontal scurry, stompable/shootable rules, and falling death physics.
- **Parallax Visuals & Retro UI:** Half-speed scrolling outdoor clouds/tubes (with animated fire) and indoor brick backdrops, original C64 title banner (`Tiles.png`), `Interface.png` menu icons, animated HUD coin, Story & Credits screens, Pause overlay, and Game Over / Victory screen.
- **Native Audio & Persistent Options:** All gameplay WAV SFX (including floppy `disk.wav`) + native looping `.it` tracker music (`title.it`, `outdoors.it`, `indoors.it`, `highscore.it`), persistent `base/Options.oi` read/write, resolution switching (`640x480`, `800x600`, `1280x720`), fullscreen toggle, global volume hotkeys, and gamepad support.
- **OpenGGS Maker Tools:**
  - **Random Level Generator:** Procedural stage assembly from `base/stages/modules.lvl` (outdoor, indoor, and exit modules + enemies + safe spawns), toggleable in Options or on-demand inside the Stage Editor.
  - **Stage Editor:** Place multi-tile elements or single tiles, place/remove enemies, set player start, toggle outdoors/indoors & colour theme, generate random stages, test-play directly (`F5`), and save back to `.lvl` (`Ctrl+S`).
  - **Tile Properties Editor:** Inspect all 2,320 tiles of `Tiles.png`, toggle all 13 tile flags, and save `base/Tilesheetinfo.tsi` (`Ctrl+S`).

## Build, Run & Test

Set `OPENGGS_BASE_DIR=/path/to/base` if running a standalone binary outside the repository directory.

```sh
# Run unit & behavioral tests
cargo test

# Lint with clippy
cargo clippy --all-targets -- -D warnings

# Run the game
cargo run
```

## Controls

### Gameplay
- **Move:** `Arrow Left` / `Arrow Right` (or Gamepad D-Pad / Left Stick)
- **Jump (variable height):** `Space` or `Arrow Up` (or Gamepad South / D-Pad Up)
- **Fast-Fall:** `Arrow Down` (or Gamepad Down)
- **Shoot (Fireball+):** `Ctrl`, `X`, or `F` (or Gamepad West / East / RT)
- **Pause:** `P` or `ESC` (or Gamepad Start / Select)
- **Global Hotkeys:** `Alt+Enter` Fullscreen · `PageUp`/`PageDown` Music Volume · `Numpad +/-` SFX Volume
- **Stage / Debug Shortcuts:** `F2` Reload stage · `F3` / `F4` Next / Previous stage · `F7` / `F8` Increase / Decrease power-up

### Stage Editor (`Menu → Stage Editor`)
- **Scroll:** `Arrow Left`/`Right` or `A`/`D` (`Shift` fast scroll, `Up`/`Down` jump to start/end)
- **Paint / Erase:** `Left Click` / `Right Click`
- **Tool & Palette:** `Tab` switch tool (`Element` / `Single Tile` / `Enemy` / `Start Pos`) · `Q`/`E` cycle selection
- **Stage Settings:** `B` toggle Outdoors/Indoors · `T` cycle colour theme · `G` generate random stage · `F3`/`F4` switch stage slot
- **Save & Test:** `Ctrl+S` save to `.lvl` · `F5` test-play immediately · `ESC` return to Menu

### Tile Properties Editor (`Options → Edit Tiles`)
- **Select Flag:** `Tab`, `Q`/`E`, or `Arrow Left`/`Right`
- **Scroll Sheet:** `Arrow Up`/`Down` or `W`/`S`
- **Toggle Flag / Save:** `Left Click` on tile · `Ctrl+S` save `Tilesheetinfo.tsi` · `ESC` return to Options
