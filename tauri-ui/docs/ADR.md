# Architecture Decision Records (ADR)

## ADR-001: Use Tauri Instead of Electron

**Date**: 2025-11-07
**Status**: Accepted
**Decision Makers**: System Architecture Designer

### Context

The Swictation UI needs a cross-platform desktop application framework. Two primary options were considered:
1. Electron (Chromium + Node.js)
2. Tauri (Rust + Web View)

### Decision

**We will use Tauri as the application framework.**

### Rationale

**Performance:**
- Tauri apps are 10-20x smaller (3-5 MB vs 50-100 MB)
- Lower memory footprint (Rust backend vs Node.js)
- Native system webview instead of bundled Chromium
- Critical for a background/system tray app

**Security:**
- Rust's memory safety prevents common vulnerabilities
- No Node.js runtime attack surface
- Explicit file system access controls
- Smaller codebase to audit

**Resource Efficiency:**
- Important for dictation app running continuously
- Lower CPU/memory usage for metrics monitoring
- Better battery life on laptops

**Trade-offs:**
- Smaller ecosystem than Electron
- Less mature (but stable for production since v1.0)
- Requires Rust knowledge for backend work

### Consequences

**Positive:**
- Users get a lightweight, responsive app
- Lower system resource consumption
- Better security posture
- Faster startup time

**Negative:**
- Team needs Rust expertise
- Fewer third-party libraries
- Platform-specific webview differences (rare)

---

## ADR-002: Use React with TypeScript for Frontend

**Date**: 2025-11-07
**Status**: Accepted

### Context

Need to choose a frontend framework for the UI. Options considered:
1. React + TypeScript
2. Vue + TypeScript
3. Svelte
4. Vanilla JS

### Decision

**We will use React 18 with TypeScript.**

### Rationale

**Developer Experience:**
- Large ecosystem of components and tools
- Excellent TypeScript support
- Strong community and documentation

**Type Safety:**
- TypeScript prevents runtime errors
- Better IDE support and autocomplete
- Self-documenting code

**Component Reusability:**
- Easy to create modular, reusable components
- Hooks provide clean state management
- No external state library needed for this app's complexity

**Performance:**
- React 18 concurrent features
- Automatic batching for updates
- Sufficient for metrics dashboard needs

### Consequences

**Positive:**
- Type-safe frontend code
- Rich ecosystem for charts, UI components
- Easy to find developers familiar with React

**Negative:**
- Slightly larger bundle size than Svelte
- Learning curve for React-specific patterns

---

## ADR-003: Use SQLite for Data Storage (Read-Only Access)

**Date**: 2025-11-07
**Status**: Accepted

### Context

The UI needs to display data written by the Swictation daemon. The daemon already uses SQLite. We need to decide how the UI accesses this data.

### Decision

**The UI will read directly from the SQLite database created by the daemon.**

Access pattern:
- Read-only access
- No schema modifications
- No writes from UI
- Daemon owns the database

### Rationale

**Simplicity:**
- No need for separate storage
- No data synchronization issues
- Single source of truth

**Performance:**
- SQLite is optimized for read queries
- Local file access is fast
- No network overhead

**Reliability:**
- Mature, battle-tested database
- ACID guarantees from daemon writes
- No cache invalidation complexity

**Data Integrity:**
- Read-only access prevents corruption
- Daemon controls schema
- Clear ownership boundaries

### Consequences

**Positive:**
- Simple architecture
- Fast query performance
- No data duplication
- Minimal complexity

**Negative:**
- UI must handle database not existing yet
- Need to ensure read locks don't block daemon
- Cannot modify historical data from UI

**Mitigations:**
- Use short-lived connections
- Implement graceful degradation if DB unavailable
- Display clear error messages

---

## ADR-004: Use Unix Socket for Real-Time Updates

**Date**: 2025-11-07
**Status**: Accepted

### Context

The UI needs real-time updates from the daemon for:
- Current session metrics (CPU, GPU, memory, WPM)
- Live transcription feed
- Daemon state changes

Options considered:
1. Poll database at intervals
2. Unix domain socket with event stream
3. IPC message queue
4. gRPC

### Decision

**We will connect to the daemon's existing Unix socket for real-time updates.**

Socket details:
- Path: `/tmp/swictation_metrics.sock`
- Protocol: Line-delimited JSON
- Connection: Persistent with auto-reconnect

### Rationale

**Performance:**
- Near-zero latency for local IPC
- No polling overhead
- Efficient push-based updates

**Daemon Integration:**
- Socket already exists in daemon
- No protocol changes needed
- Consistent with daemon architecture

**Simplicity:**
- Standard Unix IPC mechanism
- Easy to implement in Rust
- No external dependencies

**Real-time Capability:**
- Immediate updates as events occur
- No polling delay
- Efficient for high-frequency metrics

### Consequences

**Positive:**
- Real-time UI updates
- Low CPU overhead
- Simple implementation
- Reliable connection

**Negative:**
- Need to handle socket unavailable
- Need reconnection logic
- Unix-specific (not Windows)

**Mitigations:**
- Implement automatic reconnection
- Show connection status in UI
- Graceful fallback to DB-only mode
- Future: add Windows named pipe support

---

## ADR-005: Use React Context API Instead of Redux

**Date**: 2025-11-07
**Status**: Accepted

### Context

Need state management solution for frontend. Options:
1. Redux
2. MobX
3. Zustand
4. React Context API + Hooks

### Decision

**We will use React Context API with custom hooks for state management.**

### Rationale

**Simplicity:**
- No external dependencies
- Built into React
- Simpler mental model for small app

**Right-Sized:**
- App state is not complex enough for Redux
- Mostly server state (fetched data)
- Local UI state only

**Performance:**
- Context updates are efficient for our use case
- Custom hooks provide granular subscriptions
- React 18 concurrent features help

**Developer Experience:**
- Less boilerplate than Redux
- Type-safe with TypeScript
- Easy to understand and maintain

### Consequences

**Positive:**
- Lighter bundle size
- Faster development
- Easier onboarding
- Less code to maintain

**Negative:**
- No Redux DevTools
- Manual optimization needed for complex state
- Less structured than Redux

**When to Reconsider:**
- App grows to 10+ views
- Complex state interactions emerge
- Need time-travel debugging

---

## ADR-006: Use rusqlite with Thread-Safe Connection

**Date**: 2025-11-07
**Status**: Accepted

### Context

The Rust backend needs to query SQLite database. Need to decide:
- Which SQLite crate
- Connection management strategy
- Threading model

### Decision

**We will use `rusqlite` with `Arc<Mutex<Connection>>` for thread-safe access.**

### Rationale

**rusqlite vs Diesel:**
- rusqlite is lightweight and well-maintained
- No ORM overhead needed for read-only queries
- Direct SQL gives precise control
- Diesel is overkill for this use case

**Thread Safety:**
- Tauri commands run on different threads
- Need shared connection across commands
- Arc allows shared ownership
- Mutex ensures safe concurrent access

**Single Connection:**
- Read-only workload is not contention-heavy
- Avoids connection pool complexity
- Simpler error handling
- Sufficient for UI query patterns

### Consequences

**Positive:**
- Simple connection management
- Thread-safe by design
- Minimal dependencies
- Clear ownership semantics

**Negative:**
- Mutex contention on heavy load
- Single connection limits parallelism
- No connection pooling

**Future Enhancement:**
- Add connection pool if needed
- Monitor query performance
- Profile mutex contention

---

## ADR-007: Use System Tray for Background Operation

**Date**: 2025-11-07
**Status**: Accepted

### Context

Dictation app runs continuously. UI needs to be accessible but not intrusive.

### Decision

**App will run primarily in system tray with the following behavior:**
- Hide to tray on window close
- Left-click tray icon to show window
- Right-click for menu options
- Quit option in menu

### Rationale

**User Experience:**
- Common pattern for background apps
- Quick access when needed
- Doesn't clutter taskbar
- Familiar behavior

**Resource Efficiency:**
- Window can be hidden to save GPU
- App continues monitoring in background
- Tauri optimizes hidden windows

**Integration:**
- Built into Tauri
- Easy to implement
- Platform-native menus

### Consequences

**Positive:**
- Non-intrusive background operation
- Quick access via tray
- Familiar UX pattern
- Lower resource usage when hidden

**Negative:**
- Users might not notice it's running
- Need clear indication of status

**Mitigations:**
- Show notification on first run
- Tray icon indicates state
- Documentation explains behavior

---

## ADR-008: Use Vite for Build Tooling

**Date**: 2025-11-07
**Status**: Accepted

### Context

Need build tool for React + TypeScript development.

### Decision

**We will use Vite as the build tool and dev server.**

### Rationale

**Performance:**
- Instant hot module replacement (HMR)
- Fast cold start
- Optimized production builds
- ESBuild-powered

**Developer Experience:**
- Minimal configuration
- Built-in TypeScript support
- React Fast Refresh
- Pre-configured for Tauri

**Modern Standards:**
- Native ES modules
- Modern browser targets
- Tree-shaking
- Code splitting

**Tauri Integration:**
- Official Tauri + Vite templates
- Well-tested combination
- Clear documentation

### Consequences

**Positive:**
- Fast development iterations
- Quick startup time
- Excellent DX
- Future-proof

**Negative:**
- Slightly newer than Webpack
- Smaller ecosystem than Webpack

---

## ADR-009: Use Recharts for Data Visualization

**Date**: 2025-11-07
**Status**: Accepted

### Context

Need charting library for metrics visualization:
- Real-time CPU/GPU/memory charts
- Historical session WPM graphs
- Session duration timelines

Options considered:
1. Recharts
2. Chart.js
3. D3.js
4. Victory

### Decision

**We will use Recharts for all data visualization.**

### Rationale

**React Integration:**
- Built for React
- Component-based API
- Hooks support
- TypeScript types included

**Ease of Use:**
- Declarative API
- Responsive by default
- Good defaults
- Customizable when needed

**Features:**
- Line, bar, area charts
- Real-time updates
- Tooltips, legends
- Animations

**Bundle Size:**
- Reasonable size (~100 KB)
- Tree-shakeable
- Good performance

### Consequences

**Positive:**
- Fast implementation
- Consistent chart styling
- Easy maintenance
- Good documentation

**Negative:**
- Less powerful than D3
- Limited chart types
- Some customization limits

**Alternatives If Needed:**
- Switch to D3 for complex visualizations
- Use Canvas for high-frequency data

---

## ADR-010: Use date-fns for Date Handling

**Date**: 2025-11-07
**Status**: Accepted

### Context

Need date/time formatting and manipulation library.

### Decision

**We will use date-fns for date operations.**

### Rationale

**Performance:**
- Smaller than Moment.js
- Tree-shakeable (import only what you need)
- Pure functions
- No mutability issues

**TypeScript Support:**
- Excellent type definitions
- Type-safe operations

**API Design:**
- Consistent API
- Functional style
- Easy to use

**Bundle Size:**
- Only ~2-3 KB for basic formatting
- vs 67 KB for Moment.js

### Consequences

**Positive:**
- Smaller bundle size
- Better performance
- Type-safe date operations
- Modern codebase

**Negative:**
- Less familiar than Moment
- Slightly more verbose

---

## Summary of Key Decisions

| Decision | Technology | Rationale |
|----------|-----------|-----------|
| App Framework | Tauri 1.5 | Performance, security, resource efficiency |
| Frontend | React 18 + TypeScript | Type safety, ecosystem, developer experience |
| Backend | Rust | Memory safety, performance, Tauri integration |
| Database | SQLite (read-only) | Simple, fast, existing daemon database |
| Real-time | Unix Socket | Low latency, existing daemon socket |
| State Management | React Context API | Right-sized, simple, no deps |
| Build Tool | Vite | Fast dev server, modern, Tauri integration |
| Charts | Recharts | React-native, easy to use, good features |
| Dates | date-fns | Small bundle, tree-shakeable, type-safe |

---

**Last Updated**: 2025-11-07
**Version**: 0.1.0
**Format**: Based on Michael Nygard's ADR format
