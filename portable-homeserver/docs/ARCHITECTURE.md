# Portable Pubky Homeserver – Architecture Plan

This document captures the target folder structure and software architecture for the
`portable-homeserver` crate as we grow the desktop experience beyond the initial
prototype. It complements the README by describing how we intend to organize code
and responsibilities.

## High-level goals

- **Separation of concerns.** Keep UI rendering, application state, configuration
  management, and runtime orchestration in focused modules so that changes remain
  localized.
- **Testability.** Ensure pure data manipulation (e.g. configuration parsing) lives
  outside of UI code and can be unit-tested without the desktop shell.
- **Extensibility.** Structure the tree so that we can introduce additional
  services (metrics, background jobs, telemetry) without rewriting the main
  module.
- **Portability.** Maintain the ability to run the same orchestration logic from
  non-desktop contexts (CLI, future mobile shells) by isolating side-effectful
  boundaries.

## Target folder layout

```
src/
  main.rs                 # Thin entrypoint that delegates to the application module
  app/
    mod.rs                # Module wiring and public surface for launching the UI
    bootstrap.rs          # Dioxus launch configuration and window wiring
    style.rs              # Static CSS/theme resources and lazy-loaded assets
    state.rs              # Domain state definitions and start specification helpers
    config.rs             # Config form representation, persistence, and validation
    status.rs             # Presentation helpers for server status summaries/details
    tasks.rs              # Async orchestration helpers (start/stop, background tasks)
    ui.rs                 # Dioxus components (App shell, status panel, event handlers)
  domain/                 # (future) pure business logic, shared between UI and services
  services/               # (future) wrappers around homeserver/testnet process control
  infrastructure/         # (future) adapters for logging, telemetry, persistence extras
assets/
  ...                     # Static media used by the UI
```

The `domain`, `services`, and `infrastructure` folders will be introduced as soon
as features require them. Keeping placeholders in the plan ensures contributors
have a shared mental model when adding new code.

## Module responsibilities (current)

- `app::bootstrap` – Encapsulates Dioxus launch configuration so `main.rs` stays
  trivial.
- `app::style` – Owns CSS and asset loading; guarantees that theming changes do not
  leak into logic-heavy modules.
- `app::state` – Describes the enums and structs that represent runtime server
  state, network selection, and validation errors. It also exposes
  `resolve_start_spec` for mapping user intent to runtime actions.
- `app::config` – Translates between `ConfigToml` and UI-facing `ConfigForm`,
  handles disk I/O for configuration, and exposes helpers for manipulating form
  state within signals.
- `app::status` – Provides view-model helpers (`StatusCopy`, `StatusDetails`) for
  rendering user-friendly status panels.
- `app::tasks` – Runs async orchestration (start/stop homeserver, spawn testnet)
  and mediates shared state updates.
- `app::ui` – Contains the Dioxus components (`App`, `StatusPanel`) and ties
  signals, config helpers, and task orchestration together.

## Upcoming layers

1. **Domain abstractions.** As we implement backup/restore or account management,
   introduce `src/domain/` for pure models and logic (e.g. key management). This
   layer must remain free of Dioxus dependencies.
2. **Service providers.** Extract homeserver orchestration, logging setup, and
   testnet bootstrapping into `src/services/` so the UI triggers behaviors through
   interfaces. This will make it easier to share orchestration with a CLI.
3. **Infrastructure adapters.** For concerns like telemetry or IPC, provide
   concrete implementations inside `src/infrastructure/` with traits exposed to
   higher layers.
4. **Feature modules.** When adding new UI sections (backup wizard, diagnostics,
   etc.) create dedicated modules under `src/app/` (e.g. `app/diagnostics.rs`) and
   import them from `app::ui`.

## Testing strategy

- Continue writing unit tests next to logic-heavy modules (`config`, `status`,
  `state`, `tasks`). UI components should remain thin and only orchestrate these
  helpers.
- For integration scenarios (starting/stopping services), introduce async tests
  under `tests/` once we have deterministic mocks. Until then, keep orchestration
  covered through unit tests using small helpers.
- Consider adding snapshot tests for rendered Dioxus components once the layout
  stabilizes; house them under a future `tests/ui` module to avoid coupling to
  logic tests.

## Developer workflow

- New features should start by extending the relevant domain/service module, then
  update `app::ui` to call into that logic.
- CSS or asset changes belong exclusively in `app::style` (or future theme
  modules) to keep diffs focused.
- When touching async orchestration, prefer extending `app::tasks` so that
  concurrency concerns stay centralized.

This plan will evolve as we implement additional features. Contributors are
encouraged to update the document whenever a major architectural decision is
made.
