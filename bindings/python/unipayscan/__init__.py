"""Universal Payment QR: deterministic, local-first payment QR/address parsing.

Thin Python wrapper over the native `_native` extension (PyO3 bindings compiled from the same
Rust core every other SDK in this project uses - see `crates/core`). No network calls, no
credentials, no payment execution: this only recognizes, validates, and normalizes a payload
you already have (from your own QR decoder, a pasted string, whatever) into a PaymentIntent dict.

    >>> from unipayscan import parse_payment_qr
    >>> result = parse_payment_qr("upi://pay?pa=merchant%40bank&am=499.00&cu=INR")
    >>> result["scheme"]
    'upi'

Selectively enable schemes the same way the JS/Rust SDKs do:

    >>> from unipayscan import create_scanner
    >>> scanner = create_scanner(schemes={"upi": True, "bitcoin": False})
    >>> scanner.scan("bitcoin:1BoatSLRHtKNngkdXEeobR76b53LETtpyT")["supported"]
    False
"""

from __future__ import annotations

import json
from typing import Any, Dict, List, Optional

from . import _native

__all__ = [
    "__version__",
    "Scanner",
    "create_scanner",
    "parse_payment_qr",
    "detect_payment_qr",
    "get_capabilities",
    "get_supported_schemes",
]

__version__: str = _native.version()

# Maps Python kwarg names to the CapabilityPolicy JSON keys (camelCase - see crates/core/src/policy.rs).
_POLICY_KEY_ALIASES = {"max_payload_bytes": "maxPayloadBytes"}


def _policy_to_json(policy: Optional[Dict[str, Any]]) -> Optional[str]:
    if not policy:
        return None
    normalized = {_POLICY_KEY_ALIASES.get(key, key): value for key, value in policy.items()}
    return json.dumps(normalized)


def parse_payment_qr(payload: str, policy: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
    """Parses `payload` (a string already decoded from a QR image, or pasted/typed directly).

    Returns a normalized PaymentIntent dict - see docs/schema.md in the repository. `policy`
    selectively enables/disables schemes; omit it to use the default (all `stable`/`beta` schemes
    enabled). A recognized-but-disabled scheme is still returned with `recognized: True` and
    `supported: False`, never silently dropped.
    """
    return json.loads(_native.parse(payload, _policy_to_json(policy)))


def detect_payment_qr(payload: str, policy: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
    """Cheap detection only (scheme id + confidence), without full parsing/validation."""
    return json.loads(_native.detect(payload, _policy_to_json(policy)))


def get_capabilities(policy: Optional[Dict[str, Any]] = None) -> List[Dict[str, Any]]:
    """Every registered scheme's metadata (id, maturity, features, references, ...), regardless
    of whether `policy` enables it. Use this to build a scheme-toggle UI."""
    return json.loads(_native.schemes(_policy_to_json(policy)))


def get_supported_schemes(policy: Optional[Dict[str, Any]] = None) -> List[Dict[str, Any]]:
    """`get_capabilities()` filtered to schemes at `stable` or `beta` maturity."""
    return [s for s in get_capabilities(policy) if s.get("maturity") != "experimental"]


class Scanner:
    """A parser bound to one fixed policy - construct via `create_scanner()`, not directly."""

    def __init__(self, policy: Optional[Dict[str, Any]] = None) -> None:
        self._policy_json = _policy_to_json(policy)

    def scan(self, payload: str) -> Dict[str, Any]:
        return json.loads(_native.parse(payload, self._policy_json))

    def detect(self, payload: str) -> Dict[str, Any]:
        return json.loads(_native.detect(payload, self._policy_json))

    def schemes(self) -> List[Dict[str, Any]]:
        return json.loads(_native.schemes(self._policy_json))


def create_scanner(
    *,
    preset: Optional[str] = None,
    schemes: Optional[Dict[str, bool]] = None,
    categories: Optional[Dict[str, bool]] = None,
    countries: Optional[Dict[str, Dict[str, bool]]] = None,
    max_payload_bytes: Optional[int] = None,
) -> Scanner:
    """Builds a `Scanner` bound to a capability policy. All arguments are optional and match the
    JS SDK's `createScanner({...})` shape - e.g. `preset="india"`, or
    `schemes={"upi": True, "bitcoin": False}`. Detection always stays global; this only controls
    what your application accepts as `supported`."""
    policy: Dict[str, Any] = {}
    if preset is not None:
        policy["preset"] = preset
    if schemes is not None:
        policy["schemes"] = schemes
    if categories is not None:
        policy["categories"] = categories
    if countries is not None:
        policy["countries"] = countries
    if max_payload_bytes is not None:
        policy["max_payload_bytes"] = max_payload_bytes
    return Scanner(policy or None)
