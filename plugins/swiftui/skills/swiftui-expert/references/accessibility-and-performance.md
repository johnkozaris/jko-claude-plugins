# Accessibility and performance

Accessibility and performance require runtime evidence, not a source-only
checklist.

Interactive controls need meaningful accessible names, values, traits, focus
order, and sufficiently large targets. Test Dynamic Type, VoiceOver, Reduce
Motion, keyboard navigation on Mac, long localization, and important disabled
or error states. An accessibility identifier helps automation but does not
replace a user-facing label.

Preserve stable identity in collections and avoid expensive work in `body`.
Before changing view structure for performance, measure with the current Xcode
SwiftUI instruments and inspect the Cause & Effect data available in the
installed toolchain. A high body-call count is evidence to investigate, not
proof of a bug by itself.

Use lazy containers for genuinely large or scrolling content, but do not assume
they repair unstable IDs or expensive child construction. Profile realistic
data, device classes, window sizes, and release builds.

When pixels matter, validate the running application with the appropriate
platform tool. Do not infer contrast ratios, clipping, animation quality, or
focus behavior solely from modifier names.
