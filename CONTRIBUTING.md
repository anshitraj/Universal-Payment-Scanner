# Contributing

Thank you for improving universal payment QR interoperability.

Before opening a pull request:

1. Link the public specification used by the adapter.
2. Add synthetic/public fixtures and negative cases.
3. Run the full commands in the README.
4. Document any unverified field or proprietary provider dependency.
5. Confirm that no real customer payload, secret, private key, access token, or personal payment
   identifier is committed.

Small scheme modules are preferred over changes to registry orchestration. Security reports should
follow `SECURITY.md` rather than public issues.
