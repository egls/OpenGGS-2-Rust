# OpenGGS-2-Rust

A Rust rewrite of **OpenGGS2** — a Free Software/Open Source remake of *The Great Giana Sisters*, with original music by **puch666** and original C++/SDL2 implementation by **Roman Högg** (<https://sourceforge.net/projects/openggs/>).

## Rust / Bevy Port (`rust-lib/`)

The active migration (`migration-parallel-bevy`) is built in Rust using **Bevy 0.15** and pure-Rust Impulse Tracker playback (**`xmrs` / `xmrsplayer`**), reading the original OpenGGS2 binary assets directly from `base/` (`*.lvl`, `Tilesheetinfo.tsi`, `Options.oi`, spritesheets, WAV SFX, and `.it` tracker music).

### Implemented Features
- **Full Stage & Pack Support:** Loads all 33 classic stages + 4 bonus substages (`classic.lvl`, `new.lvl`, `modules.lvl`) with stage transitions, checkpoints, warp stones, and substage entrances/exits.
- **Physics & Interactive World:** Fixed-timestep movement with coyote time, moving platforms, crumbling ground, animated tiles (coins, blocks, flames, lava), stage colour themes, collectible coins (100 coins = +1 life), coin blocks, breakable bricks, and power-up blocks.
- **Power-Ups & Combat:** Normal → Punk morphing (brick smashing) → Fireball → Ricochet → Homing → Fire Shield → Extra Life.
- **Enemies:** All 15 C++ enemy types with camera proximity activation, cliff-edge reversal, flying, leaping, special patrol paths, and stompable/shootable rules.
- **Native Audio:** All gameplay WAV SFX + native looping playback of original Impulse Tracker music (`title.it`, `outdoors.it`, `indoors.it`, `highscore.it`) with live SFX/Music volume controls.
- **Menus & UI:** Main menu with stage pack selector, Options screen, Pause overlay, HUD (score, coins, mode, lives, stage), and Game Over / Victory screen.

### Build, Run & Test

```sh
cd rust-lib

# Run unit tests
cargo test

# Run the game
cargo run
```

### Controls
- **Move:** `Arrow Left` / `Arrow Right`
- **Jump:** `Space` or `Arrow Up`
- **Shoot (Fireball+):** `Ctrl`, `X`, or `F`
- **Pause:** `P` or `ESC`
- **Stage / Debug Shortcuts:** `F2` Reload stage · `F3` / `F4` Next / Previous stage · `F7` / `F8` Increase / Decrease power-up
