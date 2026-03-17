# Refactor Plan: `ec-client` TUI Architecture

## 1. Context & Rationale

The current `ec-client` TUI is organized around technical layers rather than feature domains. 
- All screen-rendering logic lives in a flat `src/screen/` directory (40+ files).
- The central application state (`src/app/state.rs`) acts as a "God object", holding nearly 200 fields for every possible screen's transient state (input buffers, cursor positions, modal statuses).
- The central action enum (`src/app/action.rs`) defines ~200 variants.
- The update handler (`src/app/update.rs`) is a massive monolith matching over all possible actions and mutating the flat state.

This violates the project's architectural guidelines, which mandate:
> "agents shall avoid monolithic source files when feature-oriented submodules are clearer"

To keep the `ec-client` source files lean, mean, and DRY, we must reorganize the application into feature-oriented, domain-driven submodules. This reduces coupling, makes it easier to navigate, and isolates state and logic according to the game's actual domains (e.g., Fleet, Planet, Starbase).

## 2. Proposed Domain-Driven Architecture

Instead of grouping by `app/` and `screen/`, the codebase will be partitioned into domain submodules. The `src/app/` module will shrink to become a thin router and dispatcher.

**Target Directory Structure:**
```text
rust/ec-client/src/
├── app/                  # Top-level routing, dispatch, and main loop
│   ├── mod.rs            # Top-level App struct holding domain sub-states
│   ├── action.rs         # Top-level Action enum wrapping domain actions
│   ├── update.rs         # Thin dispatcher for the update loop
│   └── state.rs          # Significantly reduced AppState
├── domains/              # Feature-oriented submodules
│   ├── fleet/            # Fleet management, orders, review, transport
│   │   ├── mod.rs        # FleetState, FleetAction, update()
│   │   ├── views.rs      # Rendering dispatch for fleet screens
│   │   └── screens/      # Moved from src/screen/: fleet_menu.rs, fleet_order.rs, etc.
│   ├── planet/           # Planet database, build, commission, transport
│   │   ├── mod.rs        # PlanetState, PlanetAction, update()
│   │   ├── views.rs      # Rendering dispatch for planet screens
│   │   └── screens/      # Moved from src/screen/: planet_menu.rs, planet_build.rs, etc.
│   ├── starbase/         # Starbase operations
│   ├── empire/           # Profile, Status, Rankings, Enemies
│   ├── messaging/        # Compose Message, Reviewables
│   ├── starmap/          # Full and partial starmap views
│   └── startup/          # Main Menu, First Time setup, Splash, Reports
├── model.rs              # Shared data models and session context
├── terminal/             # TUI terminal backend integration
└── theme.rs              # Shared styling and layout
```

## 3. State Splitting & Struct Redesign

The `App` struct in `state.rs` currently holds state for all screens simultaneously. This will be refactored to encapsulate domain-specific state.

**Example Redesign:**

```rust
// In src/app/state.rs
pub struct App {
    pub core_data: CoreGameData,
    pub session: SessionContext,
    pub current_screen: ScreenId,
    // Delegated domain states:
    pub fleet_state: FleetState,
    pub planet_state: PlanetState,
    pub startup_state: StartupState,
    // ...
}

// In src/domains/fleet/mod.rs
pub struct FleetState {
    pub cursor: usize,
    pub scroll_offset: usize,
    pub list_mode: FleetListMode,
    pub order_input: String,
    pub status_message: Option<String>,
    // Other fleet-specific transient UI state
}
```

*Note on DRY best practices:* Transient UI state (like input buffers or scroll offsets) should ideally be cleared when exiting a domain, or encapsulated within a nested `enum` if only one sub-screen can be active at a time, keeping memory usage explicit and intentional.

## 4. Action and Update Delegation

The massive `Action` enum will be broken down into nested, domain-specific enums.

**Example Redesign:**

```rust
// In src/app/action.rs
pub enum Action {
    App(AppAction),
    Fleet(FleetAction),
    Planet(PlanetAction),
    Startup(StartupAction),
    // ...
}

// In src/domains/fleet/mod.rs
pub enum FleetAction {
    OpenMenu,
    OpenList(FleetListMode),
    SelectFleet(usize),
    SubmitOrder(String),
    MoveCursor(i8),
}
```

The massive match block in `src/app/update.rs` will be replaced with a thin dispatcher:

```rust
// In src/app/update.rs
pub fn update(app: &mut App, action: Action) {
    match action {
        Action::App(act) => handle_app_action(app, act),
        Action::Fleet(act) => domains::fleet::update(&mut app.fleet_state, &app.core_data, act),
        Action::Planet(act) => domains::planet::update(&mut app.planet_state, &app.core_data, act),
        // ...
    }
}
```

## 5. View/Render Delegation

Currently, `App::render()` uses a massive `match self.current_screen` block. This logic will be distributed to the domains.

**Example Redesign:**

```rust
// In src/app/state.rs (inside App impl)
pub fn render(&mut self, terminal: &mut dyn Terminal) -> Result<(), Box<dyn Error>> {
    let mut playfield = match self.current_screen {
        ScreenId::Fleet(fleet_screen) => {
            domains::fleet::views::render(&self.fleet_state, &self.core_data, fleet_screen)?
        },
        ScreenId::Planet(planet_screen) => {
            domains::planet::views::render(&self.planet_state, &self.core_data, planet_screen)?
        },
        // ...
    };
    // Draw global modals/notices
    terminal.render(&playfield)
}
```

## 6. Implementation Strategy

To avoid breaking the project, the refactor should be executed incrementally in phases. Ensure `cargo test` passes after each phase.

### Phase 1: Establish Domain Boundaries
1. Create `src/domains/` and its subdirectories.
2. Move the files from `src/screen/` into their respective `src/domains/<domain>/screens/` folders.
3. Update `pub mod` declarations and `use` statements globally to fix compilation. Do not change any logic yet.

### Phase 2: Action Splitting
1. Create the domain-specific `Action` enums (e.g., `FleetAction`, `PlanetAction`).
2. Update the main `app::Action` to wrap these enums.
3. Update keybindings in `terminal/` and view emitters to return the new nested actions.
4. Update `app::update` to unwrap the nested actions before processing them in the existing giant match block.

### Phase 3: State Extraction
1. Define localized state structs (`FleetState`, `PlanetState`, etc.) in each domain module.
2. Iteratively extract fields from `app::state::App` into these localized structs.
3. Update all references in `app::update` and `app::render` to access state via `app.fleet_state.cursor` instead of `app.fleet_cursor`.

### Phase 4: Update Delegation
1. Extract logic from the giant `match action` block in `app::update.rs` into new domain-specific `update()` functions (e.g., `domains::fleet::update`).
2. Pass only the relevant domain state and core game data to these sub-updaters.

### Phase 5: Render Dispatching
1. Group `ScreenId` enum variants by domain.
2. Extract the `render()` logic from the giant `match` in `App::render()` into domain-specific `views::render()` functions.
3. Verify that the client operates identically to its pre-refactor state.