# Visual system

Use this reference when the code exposes repeated styling, animation, layout,
or current-platform material decisions.

Extract tokens when repetition creates drift or when the product needs
systematic theming. Do not create a design-system abstraction for every
one-off value. Preserve semantic typography and Dynamic Type rather than
encoding a screenshot in fixed sizes and offsets.

Animation should explain state change. Respect Reduce Motion and avoid
unbounded, decorative, or layout-thrashing animation. Use current framework
APIs supported by the deployment target and measure complex transitions in the
running app.

System materials and effects belong where they improve hierarchy and
interaction, usually chrome rather than every content surface. Avoid nested
translucency, illegible text, and decoration that competes with the product.
Compatibility or custom chrome can be a deliberate product choice; verify
current SDK behavior before treating it as permanent.

Manual offsets and geometry readers are not automatically wrong. Flag them when
they encode a relationship that breaks under localization, resizing, or Dynamic
Type and a semantic layout primitive would express the intent more reliably.
