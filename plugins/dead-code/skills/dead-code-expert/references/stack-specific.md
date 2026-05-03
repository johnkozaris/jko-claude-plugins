# Stack-Specific Dead Code

Per-stack patterns that source-symbol detection misses. Many of these are framework- or build-system-level: a symbol is "alive" in code but the wiring around it is dead, or vice versa.

Use this reference _in addition to_ `detection-catalog.md` and `language-tools.md`. The categories here are **stack-specific dead code**, not general categories.

---

## SwiftUI / iOS

Beyond `periphery` and Xcode warnings, look for:

| Pattern                                                  | What to look for                                                                                                                                             | Detection                                                                                                                                                                                                                |
| -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Dead IBOutlets / IBActions                               | `.storyboard` / `.xib` references symbols by string. Renamed/deleted Swift symbol leaves a broken connection; orphan outlet leaves an unreferenced property. | `rg 'customClass="(\w+)"' Resources/ -or '$1'` then check each class exists. Inverse: find `@IBOutlet` / `@IBAction` properties not referenced in any `.storyboard`/`.xib` (parse XML for `destination=` / `selector=`). |
| Unused asset catalog entries                             | `Assets.xcassets/Foo.imageset/` exists but no `Image("Foo")` / `UIImage(named: "Foo")` references it.                                                        | `fd -e imageset -e colorset -e symbolset Assets.xcassets` then for each `<Name>`, grep for `"<Name>"` literal in `*.swift`.                                                                                              |
| Dead localization keys                                   | `.strings` / `.xcstrings` keys never looked up via `NSLocalizedString` / `String(localized:)` / `LocalizedStringKey`.                                        | Parse `.xcstrings` JSON for keys; grep each as a string literal.                                                                                                                                                         |
| Stale `Info.plist` entries                               | Removed feature still has `NSCameraUsageDescription`, URL schemes, or document types.                                                                        | Inspect `Info.plist`; for each `NS*UsageDescription` confirm corresponding API is still called (e.g., `AVCapture` for camera).                                                                                           |
| Unused build configurations / schemes                    | `.xcodeproj` ships configs/schemes nobody runs.                                                                                                              | `xcodebuild -list` then check CI / fastlane lanes / scripts for which schemes are actually invoked.                                                                                                                      |
| Unused SPM / CocoaPods / Carthage targets                | Package added, no symbol used.                                                                                                                               | `periphery scan` will catch most; for SPM also check that each `.product(name:)` is `import`'d somewhere.                                                                                                                |
| Unused `ViewModifier` / `ButtonStyle` / `LabelStyle`     | Defined but never `.modifier()`'d / `.buttonStyle()`'d.                                                                                                      | Grep for type definition, then for any `.modifier(<Name>())` or `<Name>()` use site.                                                                                                                                     |
| Combine subscriptions to silent publishers               | `AnyCancellable` stored from a publisher that never emits (e.g., `Just` value never read, `PassthroughSubject` no one calls `.send()` on).                   | Find `let _: AnyCancellable = ... .sink { ... }` then trace whether the upstream ever produces.                                                                                                                          |
| Unused `EnvironmentKey`                                  | Custom env value declared but no `@Environment(\.myKey)` consumer.                                                                                           | Grep for `struct \w+: EnvironmentKey`, then for `\.<keyName>` access.                                                                                                                                                    |
| `PreviewProvider` / `#Preview` referencing removed views | Preview compiles only in DEBUG; can outlive deletion.                                                                                                        | Build the Previews scheme (`xcodebuild -scheme Previews build` or build with previews enabled).                                                                                                                          |

---

## Rust

Beyond `cargo clippy`, `cargo-machete`, `cargo-udeps`:

| Pattern                                           | What to look for                                                                                                | Detection                                                                                        |
| ------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| Always-on / always-off feature flags              | `#[cfg(feature = "x")]` where `x` is in default features (always on) or no caller ever enables it (always off). | `cargo tree -e features --workspace`; cross-check with `[features]` table in every `Cargo.toml`. |
| Dead target_os branches                           | `#[cfg(target_os = "windows")]` in a project with a Linux-only build matrix.                                    | Inspect CI matrix; flag branches for absent targets.                                             |
| Workspace member crates with no `path =` consumer | Crate in `members = [...]` that no other workspace crate depends on and isn't a top-level binary.               | `cargo metadata --format-version=1 \| jq '...'` to build the dep graph.                          |
| Stale `examples/`                                 | Examples in `examples/` that no longer compile against current API.                                             | `cargo build --examples` in CI.                                                                  |
| Unused trait bounds                               | `where T: Foo + Bar` where `Bar` isn't actually required by the body.                                           | `clippy::trait_duplication_in_bounds`; manual review of generic signatures.                      |
| Lifetimes that don't constrain                    | `fn foo<'a>(x: &str) -> &str` — lifetime parameter has no effect.                                               | `clippy::extra_unused_lifetimes`.                                                                |
| Over-broad visibility                             | `pub` where `pub(crate)` would do — exposes internal API as public, defeats dead-code analysis.                 | `cargo public-api` (lists public surface); compare against documented API.                       |
| `build.rs` emitting dead `cargo:rustc-cfg` flags  | `cargo:rustc-cfg=feature_x` emitted but no `#[cfg(feature_x)]` matches.                                         | Grep `build.rs` for `rustc-cfg=` outputs; cross-check `#[cfg(...)]` in source.                   |
| Dead procedural macro arms                        | Custom derive / attribute macro branches never reached for any input type in the crate.                         | Hard to detect statically; cover in tests of the macro itself.                                   |

---

## TypeScript / React / Next.js

Beyond `knip`, `eslint`:

### React-specific

| Pattern                                          | What to look for                                                                                          |
| ------------------------------------------------ | --------------------------------------------------------------------------------------------------------- |
| `useState` whose setter is never called          | Effectively a constant — replace with `const`.                                                            |
| `useEffect` with no observable effect            | Body just reads state, no DOM/network/subscription side effect, no return cleanup.                        |
| Props destructured but never read                | `function Foo({ a, b, c })` where `c` is never referenced. ESLint `react/no-unused-prop-types`.           |
| `Context.Provider` whose value nobody consumes   | Defined `createContext`, wrapped tree, but no `useContext(MyContext)` anywhere.                           |
| `useMemo`/`useCallback` with unstable deps       | Dep array contains an inline object/array literal, defeating memoization — the hook is dead optimization. |
| Render-prop / children-as-function never invoked | Component receives `children: () => ReactNode` but renders raw `children`.                                |
| Refs created but never attached                  | `useRef()` whose `.current` is never read or assigned.                                                    |
| Higher-order components wrapping nothing         | `withFoo(Component)` where `withFoo` returns the component unchanged.                                     |

### Next.js / app router

| Pattern                                                         | Detection                                                                                                                                                       |
| --------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Pages / route segments never linked                             | Walk `app/` or `pages/`, list routes, grep all `<Link href=` / `router.push(` / `redirect(`. Routes with zero references are dead unless externally bookmarked. |
| API routes with no client caller                                | Walk `app/api/` or `pages/api/`, list endpoints, grep `fetch('/api/...')` / `axios` / RPC client calls.                                                         |
| Server actions never invoked                                    | `'use server'` functions exported but never imported into a form `action={}` or `useActionState`.                                                               |
| `getServerSideProps` / `getStaticProps` returning unused fields | Returned object has keys the page never destructures.                                                                                                           |
| Middleware branches that can't fire                             | `middleware.ts` config matcher excludes paths the branch handles.                                                                                               |

### State management

| Pattern                                               | Detection                                                                 |
| ----------------------------------------------------- | ------------------------------------------------------------------------- |
| Redux actions never `dispatch`'d                      | Action creator exported, no `dispatch(actionCreator(...))` site.          |
| Reducer cases never matched                           | `case 'FOO_LOADED':` but no action emits `type: 'FOO_LOADED'`.            |
| Selectors with no `useSelector` / `useStore` consumer | `selectFoo` defined, no caller.                                           |
| Zustand/Jotai atoms with no subscribers               | `atom(0)` / `create(...)` defined but no `useAtom` / `useStore` reads it. |

### Styling

| Pattern                                        | Detection                                                                                                |
| ---------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| Tailwind custom utilities unused               | `tailwind.config.{js,ts}` extends `theme` / `plugins`; grep generated class names in `*.{tsx,jsx,html}`. |
| Tailwind safelist entries never referenced     | Safelist exists to keep dynamic classes; if dynamic source is gone, safelist is dead.                    |
| CSS modules exports unused                     | `styles.foo` defined in `.module.css`, never imported.                                                   |
| `styled-components` / `emotion` exports unused | `const Foo = styled.div\`\``defined, no`<Foo>` use.                                                      |
| `@keyframes` / `@media` blocks unused          | `animation-name` not referenced; media query for breakpoint nothing else uses.                           |

### Type-only

| Pattern                                           | Detection                                                                  |
| ------------------------------------------------- | -------------------------------------------------------------------------- |
| Unused `type` aliases / `interface`s              | `tsc --noEmit` won't flag; `knip` does.                                    |
| Generics never instantiated with non-default args | `function f<T = string>()` always called as `f()`. Type parameter is dead. |
| `interface` extensions with no implementor        | `interface Foo extends Bar` where nothing actually `implements Foo`.       |
| Discriminated union variants never constructed    | `type X = A \| B \| C` where `C` is never produced anywhere.               |

### Build / module graph

| Pattern                                                | Detection                                                                                   |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------------- |
| Barrel files re-exporting symbols nothing imports      | `index.ts` re-exports 50 symbols, only 5 are imported externally. `knip --include exports`. |
| `paths` aliases in `tsconfig.json` never used          | Walk `compilerOptions.paths`, grep each alias prefix in source.                             |
| GraphQL queries / mutations defined but never executed | `gql\`...\``template tagged but no`useQuery`/`useMutation` / direct client call.            |

---

## C# / .NET

Beyond Roslyn analyzers and ReSharper/Rider:

| Pattern                                                                          | What to look for                                                                                              | Detection                                                                                                                                           |
| -------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| Services registered but never injected                                           | `services.AddScoped<IFoo, Foo>()` but no constructor parameter / `[FromServices]` ever asks for `IFoo`.       | Walk `Program.cs` / `Startup.ConfigureServices` for `Add{Scoped,Singleton,Transient}<...>`; grep for the interface as a constructor parameter type. |
| Types constructed only by DI but never registered                                | `Foo : IFoo` exists, no `AddScoped<IFoo, Foo>()` call.                                                        | Cross-check class definitions against registrations.                                                                                                |
| EF Core dead navigation properties                                               | `public ICollection<Order> Orders { get; set; }` on `Customer` but no LINQ query traverses `.Orders`.         | Grep for `.Orders` / `.Include(c => c.Orders)`.                                                                                                     |
| `DbSet<T>` never queried                                                         | `public DbSet<Foo> Foos => Set<Foo>()` defined, no `_ctx.Foos` / `_ctx.Set<Foo>()` consumer.                  | Grep usages.                                                                                                                                        |
| Migrations for entities since deleted                                            | `Migrations/` folder still has Up/Down for tables removed from the model.                                     | Compare migration table list vs current `DbContext` `DbSet`s. **Don't delete migrations** — see `safe-removal.md`.                                  |
| Shadow properties unused                                                         | `modelBuilder.Entity<X>().Property<int>("Hidden")` defined, never read via `EF.Property<>`.                   | Grep `EF.Property<...>("Hidden")`.                                                                                                                  |
| `appsettings.json` keys never bound                                              | JSON has `"FeatureX": { ... }` but no `Configuration.GetSection("FeatureX")` / `IOptions<FeatureXOptions>`.   | Walk JSON keys, grep each as a string literal.                                                                                                      |
| `IOptions<T>` registered but no JSON populates it                                | Reverse of above.                                                                                             | Find `services.Configure<T>(...)` / `.AddOptions<T>()` then check JSON binding source.                                                              |
| Minimal API endpoints with no client caller                                      | `app.MapGet("/foo", ...)` and no client (frontend / Postman collection / OpenAPI consumer) references `/foo`. | Static check only finds server side; combine with route inventory + client grep.                                                                    |
| gRPC RPCs declared but unused                                                    | `.proto` defines `rpc Foo(...)` but neither client SDK nor service implementation references it.              | Parse `.proto`; grep generated stubs.                                                                                                               |
| `IHostedService` / `BackgroundService` no-ops                                    | `ExecuteAsync` body is just `await Task.Delay(...)` or empty loop.                                            | Read each `BackgroundService` `ExecuteAsync`.                                                                                                       |
| Middleware that doesn't read context or call `next`                              | `app.Use((ctx, next) => next())` — pure passthrough.                                                          | Grep for `app.Use(` and inspect each delegate.                                                                                                      |
| `.resx` resource entries never looked up                                         | Strings in resources file never referenced by generated property name.                                        | Parse `.resx` keys, grep the `Resources.<Key>` access.                                                                                              |
| `<ProjectReference>` / `<PackageReference>` whose namespaces are never `using`'d | Project depends on a package no source file imports.                                                          | Per project, list package namespaces (e.g., from `obj/project.assets.json`), grep `using` statements.                                               |
| Razor partials / ViewComponents never invoked                                    | `_Foo.cshtml` / `FooViewComponent` defined, no `<partial>` / `@await Component.InvokeAsync("Foo")` call.      | Grep partial/component name in `.cshtml`.                                                                                                           |
| AOT/trim warnings flagging unreachable code                                      | `IL2026`, `IL3050` warnings from `<PublishAot>` / `<PublishTrimmed>` builds.                                  | Build with `dotnet publish -c Release` and inspect warnings.                                                                                        |

---

## Python

Beyond `vulture`, `ruff F401/F841`:

| Pattern                                         | What to look for                                                                                                                                                                           | Detection                                                                                                                                                                                                                   |
| ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Coroutines never `await`'d                      | `async def foo()` called as `foo()` — returns coroutine that's silently dropped. Runtime dead code.                                                                                        | Look for warnings: `RuntimeWarning: coroutine 'foo' was never awaited`. Or grep for `async def` definitions, then for each call site verify `await` / `asyncio.create_task` / `asyncio.gather` / `loop.run_until_complete`. |
| `TYPE_CHECKING` imports left after refactor     | `if TYPE_CHECKING: from x import Y` where `Y` no longer appears in any annotation in this file.                                                                                            | Grep `Y` in file body excluding the import block.                                                                                                                                                                           |
| `Protocol` classes never structurally matched   | `class FooProto(Protocol): ...` defined, no function/variable annotated with `FooProto`.                                                                                                   | Grep type annotations.                                                                                                                                                                                                      |
| pytest fixtures no test requests                | `@pytest.fixture def foo():` in `conftest.py`, no test function takes `foo` as parameter.                                                                                                  | `pytest --fixtures-per-test` then diff against `conftest.py` definitions. `pytest-deadfixtures` plugin automates this.                                                                                                      |
| FastAPI `Depends()` declared but unused         | `def handler(x: int = Depends(get_x))` where `x` is never read in the body.                                                                                                                | Standard unused-parameter detection (vulture, ruff).                                                                                                                                                                        |
| FastAPI `response_model` types never serialized | `@app.get(..., response_model=Foo)` where `Foo` is only ever used as the response model and nothing else constructs it server-side. Probably alive (serialization use) but worth flagging. | Grep `Foo(` constructor calls.                                                                                                                                                                                              |
| Celery / RQ / Dramatiq tasks never invoked      | `@app.task` / `@task` decorator on a function, no `.delay()` / `.apply_async()` / direct enqueue site.                                                                                     | Grep task name.                                                                                                                                                                                                             |
| Click / Typer subcommands unreachable           | Subcommand registered with `@cli.command()` but no documentation, scripts, or docs reference it.                                                                                           | Run `cli --help` and cross-check against docs / shell history / Makefile.                                                                                                                                                   |
| `__all__` mismatches                            | `__all__ = ['a', 'b']` includes a symbol no longer defined, or omits a real public symbol.                                                                                                 | Compare module symbols against `__all__`.                                                                                                                                                                                   |
| Dependency manifest drift                       | Imports vs `pyproject.toml` / `requirements.txt`.                                                                                                                                          | `deptry` (recommended), `pip-check`, manual diff.                                                                                                                                                                           |
| Pydantic models with write-only fields          | `class Foo(BaseModel): bar: str` — `bar` is set on construction but no consumer accesses `obj.bar`.                                                                                        | Standard unused-field detection (harder for Pydantic; check via project-wide `\.bar\b` grep).                                                                                                                               |
| Django: stale `urlpatterns` entries             | URL pattern routes to a view that no longer exists, or view exists but URL is unreachable from any link/template.                                                                          | `python manage.py show_urls` (django-extensions) then grep usage.                                                                                                                                                           |

---

## C / C++

Beyond IWYU, cppcheck, clang-tidy:

| Pattern                                           | What to look for                                                                                                                                            | Detection                                                                                                                                             |
| ------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| Unused `#define` macros                           | `#define FOO 42` in a header, no `FOO` reference anywhere.                                                                                                  | `cppcheck --enable=unusedFunction` covers some; otherwise grep.                                                                                       |
| `#ifdef FOO` where FOO never defined              | Conditional compilation gate that is always-false — body is dead.                                                                                           | Find every `#ifdef`/`#if defined()`; check build system, command-line flags, and other headers for `#define`. `unifdef` tool can prove dead branches. |
| `#ifdef FOO` where FOO always defined             | Always-true gate — the `#else` branch is dead.                                                                                                              | Same as above.                                                                                                                                        |
| CMake `target_link_libraries` linking unused libs | Linking `Foo::Foo` but no symbol from `Foo` ever resolved.                                                                                                  | `--gc-sections` + `-Wl,--print-gc-sections` reveals unused sections at link time.                                                                     |
| Unused CMake targets                              | `add_executable(tool ...)` / `add_library(...)` never depended on by any default build target.                                                              | `cmake --build . --target help` then check what default builds.                                                                                       |
| Unused `find_package` results                     | `find_package(Foo REQUIRED)` succeeds but no `target_link_libraries(... Foo::Foo)` consumes it.                                                             | Grep `find_package` / `Foo::` linkage.                                                                                                                |
| Generated code masked                             | Qt MOC, protobuf, Cap'n Proto — generated `.cc`/`.h` files contain symbols that look dead but are linked from generated dispatch tables.                    | Exclude generated dirs (`build/`, `generated/`, `*_pb.cc`) from dead-code analysis.                                                                   |
| Pimpl with single impl                            | `class Foo { struct Impl; std::unique_ptr<Impl> p; };` where `Impl` is never swapped for testing or alternate backends. Adds a heap allocation for nothing. | Find `struct Impl` definitions; check whether `Impl` has alternative implementations or polymorphism.                                                 |
| Dead `friend` declarations                        | `friend class Bar` but `Bar` never accesses any private member.                                                                                             | Grep usages from declared friend types.                                                                                                               |
| Overloaded operators never invoked                | `operator==`, `operator<` defined but no caller (no `==` / `<` between objects of this type).                                                               | Grep usage; complicated for templates.                                                                                                                |
| `extern template` for unused types                | `extern template class Foo<int>;` exists, but `Foo<int>` never instantiated in any TU.                                                                      | Grep `Foo<int>` usage.                                                                                                                                |
| Header-include leak                               | `#include <vector>` in header where forward declaration would suffice; the include's symbols are only used in `.cpp`.                                       | IWYU flags these.                                                                                                                                     |

---

## Cross-Stack: Binary / Bundle Inspection

Independent of language, build outputs reveal dead code:

- **JS bundles**: `webpack-bundle-analyzer`, `source-map-explorer`, `rollup-plugin-visualizer` show what landed in the output. Modules contributing zero bytes were tree-shaken; modules contributing bytes you don't expect indicate retained dead code (often due to side effects).
- **Native binaries**: `bloaty` (C/C++/Rust) shows per-symbol size; symbols nobody calls but that survive linking are link-time dead code.
- **iOS apps**: `LinkMap.txt` from Xcode reveals which Swift/ObjC symbols made it into the binary; combine with periphery for source-side analysis.
- **.NET assemblies**: `ILSpy` / `dotPeek` show what trimming/AOT preserved; warnings during `PublishTrimmed` flag what couldn't be proven reachable.
- **Docker images**: `dive` shows layer-by-layer file additions; files from earlier stages that aren't `COPY --from`'d are dead.
