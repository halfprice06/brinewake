# Code signing policy

BRINEWAKE's macOS builds are signed with the maintainer's Apple Developer ID
and notarized by Apple.

Free code signing for the Windows builds is provided by
[SignPath.io](https://signpath.io), certificate by
[SignPath Foundation](https://signpath.org).

## Team roles

| Role | Who |
|---|---|
| Committers and reviewers | [halfprice06](https://github.com/halfprice06) (Daniel Price) |
| Approvers | [halfprice06](https://github.com/halfprice06) (Daniel Price) |

Every change reaches the main branch through a commit by a committer.
Windows release binaries are built by this repository's GitHub Actions
workflow from a tagged commit and are signed only after an approver approves
the signing request.

## What is signed

The Windows launcher (`BRINEWAKE.exe`) and game (`brinewake.exe`) built from
this repository. Nothing else is signed with the certificate.

Independently of Windows and Apple signing, every release manifest the
launcher installs from is signed with an ed25519 key held by the maintainer;
the launcher refuses any update whose signature, size or hash does not match.

## Privacy

BRINEWAKE has no accounts and collects no data. The launcher contacts
danprice.ai and GitHub to check for updates and download them. Online play
connects directly to the other players, asks public STUN servers (Google and
Cloudflare) for your public address, and may ask your router to open a port
(UPnP). Nothing else is sent anywhere.
