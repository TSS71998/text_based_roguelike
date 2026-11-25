<p align="center">
  <img src="./resources/title-script.svg" alt="text_based_roguelike" width="100%">
</p>
<p align="center">
  <img src="./resources/title-block.svg" alt="text_based_roguelike" width="100%">
</p>

<p align="center">
  <em>An old-school, ASCII, text-based roguelike written in Rust.</em>
</p>

<p align="center">
  <a href="https://tss71998.github.io/text_based_roguelike/"><strong>▶ Play in your browser</strong></a>
</p>

<p align="center">
  <a href="#about">About</a> •
  <a href="#getting-started">Getting Started</a> •
  <a href="#gameplay">Gameplay</a> •
  <a href="#roadmap">Roadmap</a> •
  <a href="#acknowledgements">Acknowledgements</a> •
  <a href="#license">License</a>
</p>

---

## About

**text_based_roguelike** is a classic, terminal-style roguelike written in Rust, drawn by its own small software renderer. It follows the structure of the ["Roguelike Tutorial - In Rust"](https://bfnightly.bracketproductions.com/chapter_0.html) as a starting point for learning Rust, with the long-term goal of growing into its own, more unique dungeon-crawling experience.

This project exists first and foremost as a way to learn Rust fundamentals — expect the codebase to evolve quickly as new mechanics and ideas are layered on top of the original tutorial foundation.

## Features

- Classic ASCII/glyph-based rendering
- Procedurally generated dungeons
- Turn-based movement and combat
- A small in-house engine: software rendering into one pixel buffer, no GPU or graphics library needed
- A switchable first-person view (work in progress: the view and controls are wired up, the 3D renderer is next)
 

> More features are being added as the project grows beyond its tutorial origins — see the [Roadmap](#roadmap) below.

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain) and `cargo`

### Build & Run

```bash
# Clone the repository
git clone https://github.com/TSS71998/text_based_roguelike.git
cd text_based_roguelike/text_based_roguelike_project

# Build and run in debug mode
cargo run

# Or build an optimized release binary
cargo build --release
```
### Code Layout
 
- `src/engine/` — the engine, with no game knowledge: software terminal and pixel layers, font, colours, field of view, pathfinding, random numbers, PNG and REX Paint loading, and the desktop (minifb) and web (canvas) platform layers.
- `src/render/` — how the world is drawn: `top_down.rs` (the map) and `first_person.rs`, chosen by the `ViewMode` resource.
- `src/actions.rs` — keys to player actions; first person reads movement relative to the player's `Facing`.
- `src/map/tiletype.rs` — one `TileDef` per tile type: walkability, opacity, path cost and how the 3D view should build it.
 

## Gameplay

Explore the dungeon, fight monsters, and try to survive as deep as you can — permadeath applies, as is roguelike tradition.
 
| Key | Map view | First-person view |
| --- | --- | --- |
| Arrows / WASD | Move | Up/W, Down/S: step forward/back; Left/A, Right/D: turn |
| Q E Z C | Move diagonally | Q/E: step left/right (strafe) |
| Tab | Switch to first person | Switch to the map |
| G | Pick up | Pick up |
| I / B / R | Use / drop / remove an item | same |
| 1–9 | Use a consumable | same |
| . / , | Go down / up stairs | same |
| Space | Wait a turn | same |
| Escape | Save and return to the menu | same |
| F12 | Save a screenshot (desktop) | same |
 
### Play in the Browser

The game is built to WebAssembly and published to GitHub Pages on every push to `master`:
**[tss71998.github.io/text_based_roguelike](https://tss71998.github.io/text_based_roguelike/)**. It runs in any current browser. Saving isn't available in the web version yet.

To build the web version locally:

```bash
rustup target add wasm32-unknown-unknown
# The CLI version must match the wasm-bindgen version in Cargo.lock
cargo install wasm-bindgen-cli --version 0.2.126

cargo build --release --target wasm32-unknown-unknown
wasm-bindgen target/wasm32-unknown-unknown/release/text_based_roguelike_project.wasm \
  --out-dir dist --target web --no-typescript
cp web/index.html dist/

# Serve it (browsers won't load wasm from file://), then open http://localhost:8000
python3 -m http.server --directory dist 8000
```

## Roadmap

- [ ] Expand beyond the base tutorial mechanics
- [ ] Add more enemy types and items
- [ ] Introduce a more unique world/story identity
- [ ] Polish UI/UX for terminal rendering
- [ ] Continued Optimizations

Contributions, ideas, and issue reports are welcome as this project develops.

## Acknowledgements

- Built with the [Rust Roguelike Tutorial](https://bfnightly.bracketproductions.com/chapter_0.html) by [Bracket Productions](https://bracketproductions.com/) as a foundation
- The 8x8 CP437 font (`resources/terminal8x8.png`) comes from [bracket-lib](https://github.com/amethyst/bracket-lib) (MIT), which the game was originally built on
- The basis of the engine comes from [bracket-lib](https://github.com/amethyst/bracket-lib)

## License

No license has been specified for this project yet. Until one is added, all rights are reserved by the repository owner.
