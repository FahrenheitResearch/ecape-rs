# ecape-rs

`ecape-rs` is a Rust implementation of `ecape-parcel`-style entraining parcel-path ECAPE diagnostics. The goal is direct compatibility with the public Python `ecape-parcel` package while making the calculation fast enough for profile batches, HRRR-grid diagnostics, and archive-scale research.

This crate is not a new ECAPE theory. Peters et al. provide the theory and analytic ECAPE approximation; `ecape-parcel` provides the public Peters-checked Python parcel-path implementation; `ecape-rs` provides a high-throughput Rust implementation of the same class of parcel-path calculation.

## What It Implements

- Surface-based, mixed-layer, and most-unstable parcel sources.
- Entraining and non-entraining ascent.
- Pseudoadiabatic and irreversible ascent.
- Right-moving, left-moving, mean-wind, and user-defined storm-motion modes.
- Parcel-path pressure, height, temperature, water vapor, total water, density temperature, buoyancy, CIN, LFC, EL, and integrated positive buoyancy.
- JSON runners for direct Python/Rust parity harnesses and downstream model workflows.

The sensitive meteorological primitives come from the `metrust-py` Rust crates (`metrust` and `wx-math`). ECAPE-specific parcel-path behavior lives in this crate.

## Validation Status

Current validation is centered on direct parity against Python `ecape-parcel` 1.2.2.

Highlights from the methods paper:

| Check | Result |
| --- | ---: |
| Rust no-entrainment CAPE-limit test, SB/ML/MU x pseudo/irreversible | max `|dE| < 1e-9 J kg^-1` |
| HRRR 2024-05-06 warm-sector profile, 12 configs | max `|dE| = 0.041 J kg^-1`; max `|dTrho| < 4e-5 K` |
| First event-sample sweep | `3,024 / 3,024` configs passed |
| Bounded random/stress/storm-motion v2 sweep | `5,368` comparable configs, `0` parity failures |
| v2 reference exceptions | `8`, all from one Python `ecape-parcel` mixed-layer entraining edge case where no reference path returned |

For the HRRR profile benchmark, Python `ecape-parcel` package calls took about `2.8-6.1 s` per configuration. Rust solver calls took about `0.45-0.65 ms` in the same harness, a package-call speedup of roughly `5,600-13,000x` for that benchmark. These timings separate the parcel solver from HRRR GRIB loading, profile extraction, plotting, and full-grid production.

## Methods Paper

The current methods-paper draft is included in:

- [docs/method_validation/main.pdf](docs/method_validation/main.pdf)
- [docs/method_validation/main.tex](docs/method_validation/main.tex)

The paper documents the compatibility harness, no-entrainment limit checks, compatibility fixes, target/control parity sweep, bounded random/stress/storm-motion v2 sweep, runtime separation, and current limitations.

## Build

```bash
cargo build --release
```

## Test

```bash
cargo test --release
```

The Python parity harness also requires a Python environment with `ecape-parcel`, `MetPy`, `Pint`, and `NumPy`.

Example harness invocation:

```bash
python scripts/ecape_parcel_parity_harness.py \
  --profile tests/fixtures/parity_column.json \
  --output-dir target/parity/parity_column \
  --rust-bin target/release/run_case_raw \
  --python-reps 1 \
  --rust-reps 10
```

## JSON Runner

`run_case_raw` reads one JSON payload from standard input and writes normalized parcel-path output to standard output.

```bash
cargo run --release --bin run_case_raw < input.json > output.json
```

The expected input schema is the same schema emitted by the HRRR profile probes and used by the parity harness:

- `pressure_pa`
- `height_m`
- `temperature_k`
- `dewpoint_k`
- `u_wind_ms`
- `v_wind_ms`
- parcel and ascent options
- optional user-defined storm motion

## Scope And Limitations

- `ecape-rs` targets compatibility with `ecape-parcel` behavior, including its mixed-layer parcel construction and storm-motion behavior.
- The methods paper does not claim operational forecast skill. Severe-weather products such as ECAPE-EHI and plume-object diagnostics still need independent forecast verification.
- Reference-package exceptions are tracked separately from Rust/Python parity failures. If Python `ecape-parcel` does not return a parcel path for a configuration, that row is not counted as a Rust pass or failure.

## License

MIT
