# Security

## Reporting a vulnerability

Email **support@getlingara.com** with "Security" in the subject line, and
describe the issue and the steps to reproduce it. Please do not open a public
issue for a vulnerability.

## Signing secrets

Lingara signs every request it sends your app with your app's `lgr_whsec_`
signing secret, and a kit verifies that signature before it reads anything
else. The kits never log a signing secret or a signature, at any log level; a
kit that does is a vulnerability, and we want to hear about it.

Keep signing secrets out of source control. During a rotation your app holds
two secrets, and a kit accepts a request signed by either.
