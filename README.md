# Hardware: standard parts for printCAD

A workbench package for [printCAD](https://github.com/gilbertorconde/printCAD)
that puts standard hardware into a design in a few clicks: pick a part from
the toolbar, set its standard and size in the panel, and it stands on a
body of its own, sized as the standard sizes it.

The panel opens with a drawing of the part, its measures marked beside the
parts they size; the one being edited lights up as it changes. Every
number takes a formula, as printCAD's own fields do.

## What it makes

| Family | Parts | Sized by |
| --- | --- | --- |
| Screws | socket head cap, button head, countersunk, hex bolt, low head cap, set screw | ISO 4762 / DIN 912, ASME B18.3 (inch), ISO 7380-1, ISO 10642, ISO 4017, DIN 7984, ISO 4026 |
| Nuts | hex, thin, nylon insert lock, square, T-slot | ISO 4032, ISO 4035, DIN 985, DIN 562, DIN 557; T-nuts by extrusion series |
| Washers | plain, large, spring lock | ISO 7089, ISO 7093, DIN 127 |
| Extrusions | T-slot profiles of the 20, 30 and 40 series (2020, 2040, 4080…), V-slot lips | the series' slot |
| Inserts | heat-set threaded inserts, M2 to M6, standard and short, with the hole to drive them into | common brass inserts |
| Bearings | deep groove ball bearings (608, 625, 6000…), linear bushings (LM8UU…) | their designations |
| Magnets | discs, rings, blocks | common sizes, or any |
| Rods | smooth shafts, threaded rods, lead screws, dowel pins | stock diameters |
| Springs | compression springs | wire, diameter, length and turns |

Screws carry their drive recess, and fasteners can carry a modelled
thread (off by default: a thread takes the kernel a second or two). The
kernel's boolean refuses a helical groove at some start angles for some
sizes; should a thread come out marked failed, change its **Thread start
angle**. Every table dimension can be overridden with **Custom
dimensions**. Each part's tree
entry has **Edit hardware**; selecting one in the tree opens its panel.

Scripts and agents make parts with one command and read what is offered
with another:

```lua
pc.io.github.gilbertorconde.hardware.make{part = "screw", head = "button", size = "M4", length = 16}
pc.io.github.gilbertorconde.hardware.make{part = "extrusion", profile = "2040", length = 300, along = "X"}
pc.io.github.gilbertorconde.hardware.make{part = "bearing", name = "608"}
pc.io.github.gilbertorconde.hardware.catalog{}
```

Preferences › Hardware sets what new parts start from: the thread size,
whether screws carry their recess and a modelled thread, and the
extrusion series.

## Install it

In printCAD, Preferences › Workbench packages, type this repository's
address and press Install, or install a `.pcbench` file from a release
with Install from a file…. The workbench loads at once.

The panel's drawing needs a printCAD with the diagram panel widget
(October 2026 or later).

## Build it

With [rustup](https://rustup.rs), the toolchain and the `wasm32-wasip2`
target come from `rust-toolchain.toml`:

```sh
scripts/pack.sh                                  # dist/io.github.gilbertorconde.hardware-<version>.pcbench
cargo test --target x86_64-unknown-linux-gnu     # the package's own logic
(cd tests/kernel-check && cargo run --release)   # every part built by the real kernel
```

The kernel check is a crate of its own that compiles the parts' modules
in beside the native kernel, since the package itself only links as a
component; it prints each part's volume and build time.

The SDK is read from a printCAD checkout beside this one
(`../printCAD/sdk/printcad-bench-sdk`) until the diagram widget is on
printCAD's published master; `Cargo.toml` carries the git dependency to
switch back to.

## Release it

1. Set the same version in `bench.toml` and `Cargo.toml`.
2. Commit, tag and push: `git tag v0.1.0 && git push origin v0.1.0`.

The Release workflow builds the package and publishes a GitHub release
with the `.pcbench` attached; printCAD offers it as an update.

## Layout

| Path | What it is |
| --- | --- |
| `bench.toml` | the manifest: id, version, feature kinds |
| `src/lib.rs` | the workbench: tools, commands, panels, rebuild |
| `src/parts/` | one module per family: data, panel, drawing, kernel ops |
| `src/standards.rs` | the tables the parts are sized from |
| `src/geom.rs` | revolutions, prisms, crowns and thread grooves as kernel ops |
| `src/diagram.rs` | the drawing at the top of the panel |
| `icons/` | 24×24 icons, one per tool |
| `scripts/pack.sh` | build and pack a `.pcbench` |
| `tests/kernel-check/` | builds every part through the native kernel |

## License

MIT or Apache-2.0, at your option.
