# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in awk-rs, please report it responsibly:

1. **Do NOT** open a public GitHub issue
2. Email the security concern to the maintainers
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Any suggested fixes (if you have them)

## Security Considerations

### AWK and System Commands

awk-rs supports the `system()` function and pipe operations, which execute shell commands. Users should be aware that:

- AWK programs from untrusted sources can execute arbitrary commands
- Input data can be used in shell commands if the program allows it
- The `getline` command with pipes executes shell commands

### Recommendations

1. **Review AWK scripts** before running them, especially from untrusted sources
2. **Sanitize input** when using variables in `system()` or pipe commands
3. **Use `--posix` mode** (when implemented) for stricter, safer behavior
4. **Limit permissions** when running awk-rs on sensitive systems

## Response Timeline

- **Acknowledgment**: Within 48 hours
- **Initial assessment**: Within 7 days
- **Fix development**: Depends on severity
- **Disclosure**: Coordinated with reporter

Thank you for helping keep awk-rs secure!
