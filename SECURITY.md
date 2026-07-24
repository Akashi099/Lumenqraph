# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in Lumenqraph, please report it responsibly to the maintainers instead of disclosing it publicly. This allows us to investigate and release a fix before the vulnerability is exposed to potential attackers.

### Reporting Methods

We accept security reports through:

1. **GitHub Security Advisory (Recommended)**: Use the [private vulnerability reporting feature](https://github.com/Lumen-Scribe/Lumenqraph/security/advisories) to report directly to our repository.

2. **Email**: Send details to the project maintainers at security@lumenqraph.dev with the following information:
   - A clear description of the vulnerability
   - Steps to reproduce the issue (if applicable)
   - The affected version(s)
   - Potential impact and severity assessment
   - Any suggested fixes or mitigations

### What to Include

When reporting a vulnerability, please provide:

- **Type of vulnerability** (e.g., SQL injection, XSS, authentication bypass, cryptographic weakness, DoS)
- **Location** (file path, function, or component)
- **Affected versions** (commit SHA or release version)
- **Description** of the vulnerability and its potential impact
- **Proof of concept** (code snippet or test case that demonstrates the issue)
- **Your contact information** for follow-up questions
- **Optional**: A suggested fix or patch

## Response Timeline

We commit to the following response timeline:

| Severity | Initial Response | Fix Release |
|----------|------------------|-------------|
| Critical | < 24 hours | < 7 days |
| High | < 48 hours | < 14 days |
| Medium | < 72 hours | < 30 days |
| Low | < 1 week | < 60 days |

**Severity Definitions:**

- **Critical**: Remote code execution, authentication bypass, or data exfiltration affecting production deployments
- **High**: Privilege escalation, significant data corruption, or denial of service
- **Medium**: Significant functional limitation, information disclosure, or cryptographic weakness with limited impact
- **Low**: Minor issues with workarounds or limited practical impact

## Supported Versions

Security updates are provided for:

- **Current release**: Full support for all issues
- **Previous major version**: Critical and high severity issues only
- **Earlier versions**: No guaranteed support (updates may be provided at maintainers' discretion)

Check the [releases page](https://github.com/Lumen-Scribe/Lumenqraph/releases) for version information.

## Security Best Practices

### For Operators

When deploying Lumenqraph in production:

1. **Keep dependencies updated**: Regularly run `cargo update` and monitor security advisories
2. **Rotate API keys regularly**: Use unique keys per client and monitor usage patterns
3. **Use HTTPS**: Always deploy behind TLS/HTTPS in production
4. **Secure webhook endpoints**: Validate webhook signatures (`timingSafeEqual` or equivalent constant-time comparison)
5. **Rate limiting**: Configure appropriate rate limits for your use case
6. **Database security**: Use strong authentication, network isolation, and regular backups
7. **Monitor logs**: Track authentication failures, unusual query patterns, and rate limit hits
8. **Use RPC rate limits**: Configure separate, tighter limits for expensive RPC-backed endpoints

### For Developers

When contributing to Lumenqraph:

1. **Never hardcode secrets**: Use environment variables for API keys, database URLs, and credentials
2. **Constant-time comparisons**: Use `subtle::ConstantTimeEq` or similar for all sensitive equality checks
3. **Input validation**: Validate all user inputs and external data
4. **Error handling**: Avoid leaking sensitive information in error messages
5. **Dependencies**: Audit transitive dependencies and maintain MSRV (minimum supported Rust version)
6. **Testing**: Include security-focused tests for authentication and authorization

## Cryptography

### Signature Verification

All webhook signatures use HMAC-SHA256. Verification must use constant-time comparison to prevent timing attacks:

```rust
use subtle::ConstantTimeEq;

let expected = compute_hmac_sha256(&body, &secret);
let constant_time_match = expected.ct_eq(&provided_signature);

if bool::from(constant_time_match) {
    // Signature is valid
} else {
    // Signature is invalid
}
```

### API Key Hashing

API keys are hashed using SHA-256 before database storage. Raw keys are never stored, reducing exposure if the database is compromised.

## Incident Response

If a vulnerability is confirmed:

1. A patch will be prepared and tested
2. A [GitHub Security Advisory](https://github.com/Lumen-Scribe/Lumenqraph/security/advisories) will be published with the fix
3. Users will be notified through release notes and advisories
4. The reporter will be credited (unless they request anonymity)
5. A post-mortem analysis may be conducted for critical issues

## Acknowledgments

We appreciate the security research community and thank all reporters who have responsibly disclosed vulnerabilities. Contributors to security improvements will be acknowledged in our [Security Credits](SECURITY_CREDITS.md) file (if applicable).

## Questions or Concerns?

If you have general security questions or concerns about Lumenqraph's security architecture, please open a [GitHub Discussion](https://github.com/Lumen-Scribe/Lumenqraph/discussions) or reach out to the maintainers.

---

**Last updated**: 2025-01-24

For more information on Stellar's security practices, see the [Stellar Security Resources](https://developers.stellar.org/docs/reference/security).
