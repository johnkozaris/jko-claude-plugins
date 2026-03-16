# False Positives: Code That Looks Dead But Isn't

Before removing ANY code flagged as unused, check these categories. Removing false positives breaks production.

## 1. Reflection / Metaprogramming

Code accessed by string name at runtime. Static analysis cannot see these references.

**Examples:**
- Python: `getattr(obj, "method_name")`, `importlib.import_module("package.module")`
- Java: `Class.forName("com.example.MyClass")`, `Method.invoke()`
- C#: `Type.GetMethod("MethodName")`, `Activator.CreateInstance(typeof(T))`
- Ruby: `send(:method_name)`, `const_get(:ClassName)`
- JS: `obj[methodName]()`, `require(variablePath)`

**Mitigation:** If a project uses reflection, search for string literals matching the "dead" symbol name.

## 2. Serialization / Deserialization

Fields and classes used by serializers may have zero explicit references in code.

**Examples:**
- JSON: `[JsonProperty("field")]`, `@JsonIgnore`, `#[serde(rename = "x")]`
- XML: `[XmlElement]`, `@XmlType`
- Protocol Buffers: Generated classes used only by protobuf runtime
- ORM: Entity classes used only by database mapping (Django models, EF Core entities, SQLAlchemy models)
- GraphQL: Resolver functions matched by name convention

**Mitigation:** Check if the class/field has serialization attributes or is in a models/entities directory.

## 3. Framework Magic / Convention-Based Usage

Frameworks discover and invoke code by naming convention, not explicit imports.

**Examples:**
- ASP.NET: Controllers (discovered by `*Controller` suffix), Razor Pages, Middleware
- Django: views.py functions (referenced in urls.py as strings), models (used by ORM), management commands, template tags
- Flask: `@app.route` decorated functions, template filters
- Spring: `@Component`, `@Service`, `@Bean` annotated classes
- React: Components that might only be used in JSX (detected by import, but lazy-loaded ones may not be)
- SwiftUI: `@main` App struct, `PreviewProvider` types
- Rails: Controllers, concerns, helpers (loaded by convention)
- pytest: Fixtures (loaded by name matching, not import)

**Mitigation:** Know the project's framework. Check for decorators, attributes, and naming conventions before flagging.

## 4. Plugin / Extension Points

Code designed to be loaded dynamically by a plugin system.

**Examples:**
- Entry points in `setup.py` / `pyproject.toml`
- Service providers in Laravel, Spring, .NET DI containers
- Gradle/Maven plugins loaded by configuration
- VS Code extension activation events
- Webpack loaders and plugins

**Mitigation:** Check configuration files (setup.py, pom.xml, package.json, etc.) for references to the "dead" symbol.

## 5. Public API Surface (Libraries)

If the project is a **library** (not an application), exported symbols may have zero internal callers but are used by downstream consumers.

**Indicators that code is a public API:**
- `pub` (Rust), `public` (Java/C#), `export` (JS/TS), `__all__` (Python)
- Documented in README or API docs
- Part of a published package (npm, PyPI, crates.io, NuGet)
- Has SemVer versioning

**Rule:** In libraries, only flag unused **private/internal** code as dead. Unused **public** exports are the API surface.

## 6. Lifecycle / Protocol Methods

Methods required by a protocol, interface, or base class even if not called explicitly.

**Examples:**
- Python: `__init__`, `__str__`, `__repr__`, `__enter__`, `__exit__`, `__hash__`, `__eq__`
- Java: `toString()`, `hashCode()`, `equals()`, `finalize()`, `Serializable.readObject()`
- Swift: `viewDidLoad()`, `body` (SwiftUI), `init(from:)` (Codable)
- Rust: `Drop::drop()`, `Display::fmt()`, `Default::default()`
- C#: `Dispose()`, `ToString()`, lifecycle events in Blazor/ASP.NET

**Mitigation:** Check if the method overrides a trait/interface/protocol method or is a known lifecycle hook.

## 7. Template / View References

Code referenced from HTML templates, JSX, or other non-source files that static analysis may not scan.

**Examples:**
- Django/Jinja2: `{{ variable }}`, `{% url 'view_name' %}`, template tags
- Angular: Component selectors in HTML templates
- Vue: Methods/computed properties referenced in `<template>` section
- Razor: `@Model.Property`, `@Html.Action("name")`
- SwiftUI: Property wrappers used in body

**Mitigation:** Search template files, HTML, and markup in addition to source code.

## 8. Build-Time / Code Generation

Code consumed by build tools, generators, or preprocessors.

**Examples:**
- Macro inputs (Rust proc macros, C preprocessor)
- GraphQL schema definitions consumed by codegen
- Protobuf/Thrift definitions consumed by compilers
- Swagger/OpenAPI specs consumed by generators
- Database migration files consumed by migration runners

## 9. FFI / Interop

Code called from another language across a foreign function interface.

**Examples:**
- Rust: `#[no_mangle] pub extern "C" fn` called from C
- Python: C extension functions called via ctypes/cffi
- C#: P/Invoke targets, COM-visible classes
- Swift: `@objc` methods called from Objective-C

## 10. Event Handlers Registered Declaratively

Handlers registered in config, markup, or at runtime.

**Examples:**
- HTML: `onclick="handleClick()"`, `addEventListener('click', handler)`
- Android: XML layout `android:onClick="onButtonClick"`
- iOS: Interface Builder actions/outlets
- Signal/slot connections in Qt
- Event bus subscribers (e.g., `@Subscribe` in Guava EventBus)

## Decision Framework

When uncertain, score the evidence:

| Signal | Points |
|---|---|
| Zero references in project-wide ripgrep | +2 |
| Private/internal visibility | +2 |
| No framework decorators/attributes | +1 |
| No serialization attributes | +1 |
| Old git blame (>1 year, no recent touches) | +1 |
| Has lint suppression (`#[allow(dead_code)]`) | +1 |
| In an application (not a library) | +1 |
| Has framework decorator/convention name | -3 |
| Has serialization/ORM attributes | -3 |
| Public visibility in a library | -3 |
| Recent git blame (<1 month) | -2 |

**Score 5+:** High confidence dead. Flag for removal.
**Score 2-4:** Medium confidence. Flag with investigation note.
**Score <2:** Low confidence. Do not flag without deeper analysis.
