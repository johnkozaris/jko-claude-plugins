# Networking and security

Model connectivity as a state machine with bounded retries, backoff, and an
observable terminal state. Event callbacks may be re-entrant or run on
framework-owned tasks; keep heavy work out of them and define ownership of
clients, sockets, and credentials.

Bound message sizes, receive buffers, outstanding requests, and reconnect
queues. Test loss during DNS, TLS, authentication, subscription, OTA, and
shutdown rather than only steady-state Wi-Fi.

Use certificate validation, secure credential storage, protected provisioning,
and current TLS defaults appropriate to the product. Secure boot, flash
encryption, rollback protection, signed OTA, and key rotation are system
decisions; verify chip support, efuse state, manufacturing flow, and recovery
before enabling irreversible settings.

Never log secrets or copy production credentials into firmware tests. Treat OTA
as a power-loss and partial-write problem: verify image authenticity, partition
capacity, rollback, boot confirmation, and interrupted-update recovery.

Consult current Espressif advisories and dependency versions for vulnerability
claims. Safe C++ does not prevent protocol, authorization, or resource-exhaustion
bugs.
