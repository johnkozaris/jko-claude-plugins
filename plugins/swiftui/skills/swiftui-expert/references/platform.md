# Apple-platform fit

Load this reference only when the target platform changes the answer.

For iOS and visionOS, inspect permission timing, privacy declarations,
background behavior, scene lifecycle, universal/deep links, and App Store
requirements relevant to APIs actually used. Do not paste a catalogue of every
platform integration into an unrelated review.

For macOS, evaluate menus and keyboard commands for primary actions, window and
document behavior, drag and drop, settings placement, sandbox entitlements,
distribution, updates, and AppKit interop. A SwiftUI shell with focused AppKit
bridges is normal when native controls expose behavior SwiftUI does not.

Availability, entitlement, notarization, privacy, and review requirements
change. Check the project's deployment target and current Apple documentation
before making a release-blocking claim.
